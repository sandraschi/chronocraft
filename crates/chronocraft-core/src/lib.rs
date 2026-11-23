//! ChronoCraft L1 Core Engine & Universal Command Bus.
//!
//! Pure-Rust system-dynamics simulator: stocks-and-flows graphs with feedback
//! links, a flow-rate expression evaluator, a classic RK4 integrator, a seeded
//! Monte Carlo engine producing p10/p50/p90 percentile bands, baseline-versus-
//! variant scenario compare, one-parameter sensitivity sweeps, and bundled
//! SQLite scenario persistence.
#![forbid(unsafe_code)]

pub mod db;
pub mod error;
pub mod expr;
pub mod model;
pub mod monte;
pub mod sim;

pub use db::{default_db_path, list_scenario_names, load_active, load_scenario, save_to_db};
pub use error::ChronoError;
pub use expr::{check_expr, eval_expr};
pub use model::{
    Flow, Link, ModelPreset, MonteCarloResult, Param, ScenarioDiff, SensitivityResult, ShockEvent,
    SimConfig, Stock, StockDiff, StockSeries, TimeSeries,
};
pub use monte::{run_monte_carlo, XorShift64};
pub use sim::{build_solver_model, rk4_step, run_simulation, validate_config, SolverModel};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronoEngine {
    pub active_scenario: String,
    pub stocks: Vec<Stock>,
    pub flows: Vec<Flow>,
    pub links: Vec<Link>,
    pub params: Vec<Param>,
    pub shocks: Vec<ShockEvent>,
    pub config: SimConfig,
}

impl Default for ChronoEngine {
    fn default() -> Self {
        Self::with_preset(ModelPreset::Population)
    }
}

impl ChronoEngine {
    pub fn open_or_seed(path: &Path) -> Result<Self, ChronoError> {
        if let Some(loaded) = load_active(path)? {
            return Ok(loaded);
        }
        let seeded = Self::with_preset(ModelPreset::Population);
        save_to_db(&seeded, path)?;
        Ok(seeded)
    }

    pub fn save_db(&self, path: &Path) -> Result<(), ChronoError> {
        save_to_db(self, path)
    }

    pub fn with_preset(preset: ModelPreset) -> Self {
        let mut eng = Self {
            active_scenario: preset.as_str().to_string(),
            stocks: Vec::new(),
            flows: Vec::new(),
            links: Vec::new(),
            params: Vec::new(),
            shocks: Vec::new(),
            config: SimConfig::default(),
        };
        eng.load_preset(preset);
        eng
    }

    pub fn load_preset(&mut self, preset: ModelPreset) {
        self.stocks.clear();
        self.flows.clear();
        self.links.clear();
        self.params.clear();
        self.shocks.clear();
        self.active_scenario = preset.as_str().to_string();
        match preset {
            ModelPreset::Population => {
                self.stocks.push(Stock {
                    id: "pop".into(),
                    label: "Population".into(),
                    initial: 100.0,
                    x: 320.0,
                    y: 140.0,
                });
                self.params.push(Param {
                    name: "growth".into(),
                    value: 0.1,
                    min: 0.0,
                    max: 1.0,
                });
                self.params.push(Param {
                    name: "capacity".into(),
                    value: 1000.0,
                    min: 100.0,
                    max: 5000.0,
                });
                self.flows.push(Flow {
                    id: "births".into(),
                    label: "Births".into(),
                    from_stock: None,
                    to_stock: Some("pop".into()),
                    rate_expr: "growth * pop * (1 - pop / capacity)".into(),
                    x: 160.0,
                    y: 140.0,
                });
                self.flows.push(Flow {
                    id: "deaths".into(),
                    label: "Deaths".into(),
                    from_stock: Some("pop".into()),
                    to_stock: None,
                    rate_expr: "0.02 * pop".into(),
                    x: 480.0,
                    y: 140.0,
                });
                self.links.push(Link {
                    id: "l_pop_births".into(),
                    from_node: "pop".into(),
                    to_flow: "births".into(),
                });
            }
            ModelPreset::Inventory => {
                self.stocks.push(Stock {
                    id: "goods".into(),
                    label: "Goods on Hand".into(),
                    initial: 500.0,
                    x: 320.0,
                    y: 140.0,
                });
                self.params.push(Param {
                    name: "order_rate".into(),
                    value: 40.0,
                    min: 0.0,
                    max: 200.0,
                });
                self.params.push(Param {
                    name: "demand".into(),
                    value: 35.0,
                    min: 0.0,
                    max: 200.0,
                });
                self.flows.push(Flow {
                    id: "inflow".into(),
                    label: "Inbound Orders".into(),
                    from_stock: None,
                    to_stock: Some("goods".into()),
                    rate_expr: "order_rate".into(),
                    x: 160.0,
                    y: 140.0,
                });
                self.flows.push(Flow {
                    id: "outflow".into(),
                    label: "Customer Demand".into(),
                    from_stock: Some("goods".into()),
                    to_stock: None,
                    rate_expr: "demand".into(),
                    x: 480.0,
                    y: 140.0,
                });
                self.links.push(Link {
                    id: "l_goods_outflow".into(),
                    from_node: "goods".into(),
                    to_flow: "outflow".into(),
                });
            }
            ModelPreset::EpidemicSir => {
                self.stocks.push(Stock {
                    id: "susceptible".into(),
                    label: "Susceptible".into(),
                    initial: 990.0,
                    x: 180.0,
                    y: 120.0,
                });
                self.stocks.push(Stock {
                    id: "infected".into(),
                    label: "Infected".into(),
                    initial: 10.0,
                    x: 420.0,
                    y: 120.0,
                });
                self.stocks.push(Stock {
                    id: "recovered".into(),
                    label: "Recovered".into(),
                    initial: 0.0,
                    x: 640.0,
                    y: 120.0,
                });
                self.params.push(Param {
                    name: "beta".into(),
                    value: 0.0003,
                    min: 0.0,
                    max: 0.01,
                });
                self.params.push(Param {
                    name: "gamma".into(),
                    value: 0.05,
                    min: 0.0,
                    max: 1.0,
                });
                self.flows.push(Flow {
                    id: "infection".into(),
                    label: "Infection".into(),
                    from_stock: Some("susceptible".into()),
                    to_stock: Some("infected".into()),
                    rate_expr: "beta * susceptible * infected".into(),
                    x: 300.0,
                    y: 120.0,
                });
                self.flows.push(Flow {
                    id: "recovery".into(),
                    label: "Recovery".into(),
                    from_stock: Some("infected".into()),
                    to_stock: Some("recovered".into()),
                    rate_expr: "gamma * infected".into(),
                    x: 530.0,
                    y: 120.0,
                });
                self.links.push(Link {
                    id: "l_sus_infection".into(),
                    from_node: "susceptible".into(),
                    to_flow: "infection".into(),
                });
                self.links.push(Link {
                    id: "l_inf_infection".into(),
                    from_node: "infected".into(),
                    to_flow: "infection".into(),
                });
                self.links.push(Link {
                    id: "l_inf_recovery".into(),
                    from_node: "infected".into(),
                    to_flow: "recovery".into(),
                });
            }
        }
    }

    fn known_names(&self) -> HashMap<String, f64> {
        let mut vars = HashMap::new();
        for s in &self.stocks {
            vars.insert(s.id.clone(), s.initial);
        }
        for p in &self.params {
            vars.insert(p.name.clone(), p.value);
        }
        vars.insert("t".to_string(), 0.0);
        vars
    }

    fn solver_view(&self) -> Result<SolverParts, ChronoError> {
        if self.stocks.is_empty() {
            return Err(ChronoError::Validation(
                "Model has no stocks to simulate".to_string(),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for s in &self.stocks {
            if s.id.trim().is_empty() {
                return Err(ChronoError::Validation(
                    "Stock id cannot be empty".to_string(),
                ));
            }
            if !seen.insert(s.id.clone()) {
                return Err(ChronoError::Validation(format!(
                    "Duplicate stock id '{}'",
                    s.id
                )));
            }
            if !s.initial.is_finite() {
                return Err(ChronoError::Validation(format!(
                    "Stock '{}' initial value must be finite",
                    s.id
                )));
            }
        }
        let vars = self.known_names();
        for f in &self.flows {
            if let Some(from) = &f.from_stock {
                if !self.stocks.iter().any(|s| &s.id == from) {
                    return Err(ChronoError::NotFound(format!(
                        "Flow '{}' source stock '{from}'",
                        f.id
                    )));
                }
            }
            if let Some(to) = &f.to_stock {
                if !self.stocks.iter().any(|s| &s.id == to) {
                    return Err(ChronoError::NotFound(format!(
                        "Flow '{}' target stock '{to}'",
                        f.id
                    )));
                }
            }
            eval_expr(&f.rate_expr, &vars)?;
        }
        Ok(SolverParts {
            ids: self.stocks.iter().map(|s| s.id.clone()).collect(),
            initials: self.stocks.iter().map(|s| s.initial).collect(),
            params: self
                .params
                .iter()
                .map(|p| (p.name.clone(), p.value))
                .collect(),
            shocks: self
                .shocks
                .iter()
                .map(|e| (e.stock_id.clone(), e.time, e.magnitude))
                .collect(),
        })
    }

    fn with_solver<R>(
        &self,
        f: impl FnOnce(&SolverModel<'_>) -> Result<R, ChronoError>,
    ) -> Result<R, ChronoError> {
        let parts = self.solver_view()?;
        let model = build_solver_model(
            &parts.ids,
            &parts.initials,
            &self.flows,
            &parts.params,
            &parts.shocks,
        );
        f(&model)
    }

    pub fn run(&self, steps: Option<usize>, dt: Option<f64>) -> Result<TimeSeries, ChronoError> {
        let mut cfg = self.config.clone();
        if let Some(s) = steps {
            cfg.steps = s;
        }
        if let Some(d) = dt {
            cfg.dt = d;
        }
        self.with_solver(|m| run_simulation(m, &cfg))
    }

    pub fn run_mc(
        &self,
        runs: usize,
        seed: Option<u64>,
        shock_std: Option<f64>,
    ) -> Result<MonteCarloResult, ChronoError> {
        let cfg = self.config.clone();
        let use_seed = seed.unwrap_or(cfg.seed);
        let use_std = shock_std.unwrap_or(cfg.shock_std);
        self.with_solver(|m| run_monte_carlo(m, &cfg, runs, use_seed, use_std))
    }

    pub fn add_stock(
        &mut self,
        id: &str,
        label: &str,
        initial: f64,
        x: f32,
        y: f32,
    ) -> Result<Stock, ChronoError> {
        let clean = id.trim();
        if clean.is_empty() {
            return Err(ChronoError::Validation("Stock id cannot be empty".into()));
        }
        if self.stocks.iter().any(|s| s.id == clean) {
            return Err(ChronoError::Validation(format!(
                "Stock '{clean}' already exists"
            )));
        }
        if !initial.is_finite() {
            return Err(ChronoError::Validation(
                "Stock initial value must be finite".to_string(),
            ));
        }
        let node = Stock {
            id: clean.to_string(),
            label: if label.trim().is_empty() {
                clean.to_uppercase()
            } else {
                label.trim().to_string()
            },
            initial,
            x,
            y,
        };
        self.stocks.push(node.clone());
        Ok(node)
    }

    pub fn add_flow(&mut self, spec: FlowSpec) -> Result<Flow, ChronoError> {
        let clean = spec.id.trim();
        if clean.is_empty() {
            return Err(ChronoError::Validation("Flow id cannot be empty".into()));
        }
        if self.flows.iter().any(|f| f.id == clean) {
            return Err(ChronoError::Validation(format!(
                "Flow '{clean}' already exists"
            )));
        }
        for opt in [&spec.from_stock, &spec.to_stock].into_iter().flatten() {
            if !self.stocks.iter().any(|s| &s.id == opt) {
                return Err(ChronoError::NotFound(format!(
                    "Flow stock '{opt}' does not exist"
                )));
            }
        }
        check_expr(&spec.rate_expr)?;
        // Every identifier must resolve to a stock, a parameter, or time `t`.
        let vars = self.known_names();
        // Evaluation fails on the first unknown name, which is exactly the error we want.
        eval_expr(&spec.rate_expr, &vars)?;
        let flow = Flow {
            id: clean.to_string(),
            label: if spec.label.trim().is_empty() {
                clean.to_uppercase()
            } else {
                spec.label.trim().to_string()
            },
            from_stock: spec.from_stock,
            to_stock: spec.to_stock,
            rate_expr: spec.rate_expr.trim().to_string(),
            x: spec.x,
            y: spec.y,
        };
        self.flows.push(flow.clone());
        Ok(flow)
    }

    pub fn add_link(&mut self, from_node: &str, to_flow: &str) -> Result<Link, ChronoError> {
        if !self.stocks.iter().any(|s| s.id == from_node) {
            return Err(ChronoError::NotFound(format!(
                "Link source node '{from_node}' does not exist"
            )));
        }
        if !self.flows.iter().any(|f| f.id == to_flow) {
            return Err(ChronoError::NotFound(format!(
                "Link target flow '{to_flow}' does not exist"
            )));
        }
        let id = format!("l_{from_node}_{to_flow}");
        if let Some(existing) = self.links.iter().find(|l| l.id == id) {
            return Ok(existing.clone());
        }
        let link = Link {
            id,
            from_node: from_node.to_string(),
            to_flow: to_flow.to_string(),
        };
        self.links.push(link.clone());
        Ok(link)
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> Result<Param, ChronoError> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err(ChronoError::Validation(
                "Parameter name cannot be empty".to_string(),
            ));
        }
        if !value.is_finite() {
            return Err(ChronoError::Validation(
                "Parameter value must be finite".to_string(),
            ));
        }
        if let Some(p) = self.params.iter_mut().find(|p| p.name == clean) {
            if value < p.min || value > p.max {
                return Err(ChronoError::Validation(format!(
                    "Parameter '{clean}' value {value} is outside [{}, {}]",
                    p.min, p.max
                )));
            }
            p.value = value;
            return Ok(p.clone());
        }
        let param = Param {
            name: clean.to_string(),
            value,
            min: -1e18,
            max: 1e18,
        };
        self.params.push(param.clone());
        Ok(param)
    }

    pub fn sensitivity(
        &self,
        param: &str,
        from: f64,
        to: f64,
        steps: usize,
        target: &str,
    ) -> Result<SensitivityResult, ChronoError> {
        if !self.params.iter().any(|p| p.name == param) {
            return Err(ChronoError::NotFound(format!("Parameter '{param}'")));
        }
        if !self.stocks.iter().any(|s| s.id == target) {
            return Err(ChronoError::NotFound(format!("Target stock '{target}'")));
        }
        if !(2..=21).contains(&steps) {
            return Err(ChronoError::Validation(
                "Sensitivity steps must be between 2 and 21".to_string(),
            ));
        }
        if !from.is_finite() || !to.is_finite() || from > to {
            return Err(ChronoError::Validation(
                "Sensitivity range needs finite from <= to".to_string(),
            ));
        }
        let mut values = Vec::with_capacity(steps);
        let mut end_states = Vec::with_capacity(steps);
        for i in 0..steps {
            let frac = (i as f64) / ((steps - 1) as f64);
            let v = from + frac * (to - from);
            let mut clone = self.clone();
            if let Some(p) = clone.params.iter_mut().find(|p| p.name == param) {
                p.min = p.min.min(v);
                p.max = p.max.max(v);
                p.value = v;
            }
            let trace = clone.run(None, None)?;
            values.push(v);
            end_states.push(trace.end_state(target).unwrap_or(f64::NAN));
        }
        Ok(SensitivityResult {
            param: param.to_string(),
            target: target.to_string(),
            values,
            end_states,
        })
    }

    pub fn compare_with(&self, other: &Self) -> Result<ScenarioDiff, ChronoError> {
        let base_trace = self.run(None, None)?;
        let var_trace = other.run(None, None)?;
        let mut ids: Vec<String> = base_trace.series.iter().map(|s| s.stock.clone()).collect();
        for s in &var_trace.series {
            if !ids.contains(&s.stock) {
                ids.push(s.stock.clone());
            }
        }
        let mut end = Vec::with_capacity(ids.len());
        for id in ids {
            let b = base_trace.end_state(&id).unwrap_or(f64::NAN);
            let v = var_trace.end_state(&id).unwrap_or(f64::NAN);
            end.push(StockDiff {
                stock: id,
                baseline: b,
                variant: v,
                delta: v - b,
            });
        }
        Ok(ScenarioDiff {
            baseline: self.active_scenario.clone(),
            variant: other.active_scenario.clone(),
            end,
        })
    }
}

struct SolverParts {
    ids: Vec<String>,
    initials: Vec<f64>,
    params: HashMap<String, f64>,
    shocks: Vec<(String, f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSpec {
    pub id: String,
    pub label: String,
    pub from_stock: Option<String>,
    pub to_stock: Option<String>,
    pub rate_expr: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "params", rename_all = "snake_case")]
pub enum Command {
    Status,
    Summary,
    LoadPreset {
        preset: String,
    },
    AddStock {
        id: String,
        label: String,
        initial: f64,
        x: f32,
        y: f32,
    },
    AddFlow {
        id: String,
        label: String,
        from_stock: Option<String>,
        to_stock: Option<String>,
        rate_expr: String,
        x: f32,
        y: f32,
    },
    AddLink {
        from_node: String,
        to_flow: String,
    },
    SetParam {
        name: String,
        value: f64,
    },
    RunSimulation {
        steps: Option<usize>,
        dt: Option<f64>,
    },
    RunMonteCarlo {
        runs: usize,
        seed: Option<u64>,
        shock_std: Option<f64>,
    },
    SaveScenario {
        name: String,
    },
    ListScenarios,
    CompareScenarios {
        baseline: String,
        variant: String,
    },
    SensitivitySweep {
        param: String,
        from: f64,
        to: f64,
        steps: usize,
        target: String,
    },
}

impl Command {
    pub fn dispatch(
        &self,
        engine: &mut ChronoEngine,
        db_path: &Path,
    ) -> Result<Value, ChronoError> {
        match self {
            Self::Status => Ok(json!({
                "app": "chronocraft",
                "version": APP_VERSION,
                "status": "ready",
                "active_scenario": engine.active_scenario,
                "stock_count": engine.stocks.len(),
                "flow_count": engine.flows.len(),
                "link_count": engine.links.len(),
                "param_count": engine.params.len(),
            })),
            Self::Summary => Ok(json!({
                "active_scenario": engine.active_scenario,
                "stocks": engine.stocks,
                "flows": engine.flows,
                "links": engine.links,
                "params": engine.params,
                "config": engine.config,
            })),
            Self::LoadPreset { preset } => {
                let p = ModelPreset::parse(preset).ok_or_else(|| {
                    ChronoError::Validation(format!(
                        "Unknown preset '{preset}'. Expected population, inventory, or epidemic_sir."
                    ))
                })?;
                engine.load_preset(p);
                let _ = engine.save_db(db_path);
                Ok(json!({
                    "loaded": p.as_str(),
                    "stock_count": engine.stocks.len(),
                    "flow_count": engine.flows.len(),
                }))
            }
            Self::AddStock {
                id,
                label,
                initial,
                x,
                y,
            } => {
                let node = engine.add_stock(id, label, *initial, *x, *y)?;
                let _ = engine.save_db(db_path);
                Ok(json!({ "added": node }))
            }
            Self::AddFlow {
                id,
                label,
                from_stock,
                to_stock,
                rate_expr,
                x,
                y,
            } => {
                let spec = FlowSpec {
                    id: id.clone(),
                    label: label.clone(),
                    from_stock: from_stock.clone(),
                    to_stock: to_stock.clone(),
                    rate_expr: rate_expr.clone(),
                    x: *x,
                    y: *y,
                };
                let flow = engine.add_flow(spec)?;
                let _ = engine.save_db(db_path);
                Ok(json!({ "added": flow }))
            }
            Self::AddLink { from_node, to_flow } => {
                let link = engine.add_link(from_node, to_flow)?;
                let _ = engine.save_db(db_path);
                Ok(json!({ "link": link }))
            }
            Self::SetParam { name, value } => {
                let param = engine.set_param(name, *value)?;
                let _ = engine.save_db(db_path);
                Ok(json!({ "param": param }))
            }
            Self::RunSimulation { steps, dt } => {
                let trace = engine.run(*steps, *dt)?;
                serde_json::to_value(trace).map_err(|e| ChronoError::Serialization(e.to_string()))
            }
            Self::RunMonteCarlo {
                runs,
                seed,
                shock_std,
            } => {
                let mc = engine.run_mc(*runs, *seed, *shock_std)?;
                serde_json::to_value(mc).map_err(|e| ChronoError::Serialization(e.to_string()))
            }
            Self::SaveScenario { name } => {
                let clean = name.trim();
                if clean.is_empty() {
                    return Err(ChronoError::Validation(
                        "Scenario name cannot be empty".to_string(),
                    ));
                }
                let previous = engine.active_scenario.clone();
                engine.active_scenario = clean.to_string();
                let saved = engine.save_db(db_path);
                if saved.is_err() {
                    engine.active_scenario = previous;
                }
                saved?;
                Ok(json!({ "saved": clean }))
            }
            Self::ListScenarios => {
                let names = list_scenario_names(db_path)?;
                Ok(json!({ "scenarios": names, "active": engine.active_scenario }))
            }
            Self::CompareScenarios { baseline, variant } => {
                let base = load_scenario(db_path, baseline)?
                    .ok_or_else(|| ChronoError::NotFound(format!("Scenario '{baseline}'")))?;
                let var = load_scenario(db_path, variant)?
                    .ok_or_else(|| ChronoError::NotFound(format!("Scenario '{variant}'")))?;
                let diff = base.compare_with(&var)?;
                serde_json::to_value(diff).map_err(|e| ChronoError::Serialization(e.to_string()))
            }
            Self::SensitivitySweep {
                param,
                from,
                to,
                steps,
                target,
            } => {
                let res = engine.sensitivity(param, *from, *to, *steps, target)?;
                serde_json::to_value(res).map_err(|e| ChronoError::Serialization(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn population_preset_simulates_logistic_growth() {
        let eng = ChronoEngine::with_preset(ModelPreset::Population);
        let trace = eng.run(Some(200), Some(0.05));
        assert!(trace.is_ok());
        if let Ok(ts) = trace {
            let end = ts.end_state("pop");
            assert!(end.is_some());
            // Growth with carrying capacity: end state stays above seed, below capacity.
            if let Some(v) = end {
                assert!(v > 100.0 && v < 1000.0);
            }
        }
    }

    #[test]
    fn sir_preset_conserves_total_population() {
        let eng = ChronoEngine::with_preset(ModelPreset::EpidemicSir);
        let trace = eng.run(Some(100), Some(0.05));
        assert!(trace.is_ok());
        if let Ok(ts) = trace {
            let total: f64 = ts
                .series
                .iter()
                .filter_map(|s| s.values.last().copied())
                .sum();
            assert!((total - 1000.0).abs() < 1.0);
        }
    }

    #[test]
    fn set_param_rejects_out_of_range_values() {
        let mut eng = ChronoEngine::with_preset(ModelPreset::Population);
        assert!(eng.set_param("growth", 0.5).is_ok());
        assert!(eng.set_param("growth", 5.0).is_err());
        assert!(eng.set_param("brand_new", 2.0).is_ok());
    }

    #[test]
    fn sensitivity_sweep_tracks_monotone_growth_response() {
        let eng = ChronoEngine::with_preset(ModelPreset::Population);
        let res = eng.sensitivity("growth", 0.05, 0.2, 4, "pop");
        assert!(res.is_ok());
        if let Ok(s) = res {
            assert_eq!(s.end_states.len(), 4);
            for w in s.end_states.windows(2) {
                assert!(w[1] >= w[0]);
            }
        }
    }

    #[test]
    fn sqlite_roundtrip_persists_stocks_and_params() {
        let mut path = std::env::temp_dir();
        path.push(format!("chronocraft_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);

        let eng = ChronoEngine::with_preset(ModelPreset::Inventory);
        assert!(eng.save_db(&path).is_ok());

        let loaded = load_active(&path).ok().flatten();
        let _ = std::fs::remove_file(&path);
        assert!(loaded.is_some());
        if let Some(l) = loaded {
            assert_eq!(l.active_scenario, "inventory");
            assert_eq!(l.stocks.len(), 1);
        }
    }
}
