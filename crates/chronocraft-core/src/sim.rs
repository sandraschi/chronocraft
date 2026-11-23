//! RK4 ODE solver over the stocks-and-flows graph.
//!
//! Each flow contributes its evaluated rate to its target stock and drains its
//! source stock. Flows with no source draw from outside the model; flows with
//! no target drain outside the model.

use crate::error::ChronoError;
use crate::expr::eval_expr;
use crate::model::{Flow, SimConfig, StockSeries, TimeSeries};
use std::collections::HashMap;

/// Ordered live model view used by the solver.
pub struct SolverModel<'a> {
    pub stock_ids: Vec<String>,
    pub initials: Vec<f64>,
    pub flows: &'a [Flow],
    pub params: HashMap<String, f64>,
    pub shocks: Vec<(String, f64, f64)>,
}

pub fn build_solver_model<'a>(
    stock_ids: &'a [String],
    initials: &'a [f64],
    flows: &'a [Flow],
    params: &'a HashMap<String, f64>,
    shocks: &'a [(String, f64, f64)],
) -> SolverModel<'a> {
    SolverModel {
        stock_ids: stock_ids.to_vec(),
        initials: initials.to_vec(),
        flows,
        params: params.clone(),
        shocks: shocks.to_vec(),
    }
}

/// Evaluate dS/dt for every stock at state `y` and time `t`.
pub fn derivatives(model: &SolverModel<'_>, y: &[f64], t: f64) -> Result<Vec<f64>, ChronoError> {
    let mut vars: HashMap<String, f64> = HashMap::with_capacity(y.len() + 8);
    for (id, val) in model.stock_ids.iter().zip(y.iter()) {
        vars.insert(id.clone(), *val);
    }
    for (k, v) in &model.params {
        vars.insert(k.clone(), *v);
    }
    vars.insert("t".to_string(), t);

    let mut dydt = vec![0.0_f64; y.len()];
    for flow in model.flows {
        let rate = eval_expr(&flow.rate_expr, &vars)?;
        if let Some(from) = &flow.from_stock {
            if let Some(idx) = model.stock_ids.iter().position(|s| s == from) {
                dydt[idx] -= rate;
            }
        }
        if let Some(to) = &flow.to_stock {
            if let Some(idx) = model.stock_ids.iter().position(|s| s == to) {
                dydt[idx] += rate;
            }
        }
    }
    Ok(dydt)
}

/// One classic fourth-order Runge-Kutta step.
pub fn rk4_step(
    model: &SolverModel<'_>,
    y: &[f64],
    t: f64,
    dt: f64,
) -> Result<Vec<f64>, ChronoError> {
    let k1 = derivatives(model, y, t)?;
    let y2: Vec<f64> = y.iter().zip(&k1).map(|(v, k)| v + 0.5 * dt * k).collect();
    let k2 = derivatives(model, &y2, t + 0.5 * dt)?;
    let y3: Vec<f64> = y.iter().zip(&k2).map(|(v, k)| v + 0.5 * dt * k).collect();
    let k3 = derivatives(model, &y3, t + 0.5 * dt)?;
    let y4: Vec<f64> = y.iter().zip(&k3).map(|(v, k)| v + dt * k).collect();
    let k4 = derivatives(model, &y4, t + dt)?;
    Ok(y.iter()
        .zip(&k1)
        .zip(&k2)
        .zip(&k3)
        .zip(&k4)
        .map(|((((v, a), b), c), d)| v + dt / 6.0 * (a + 2.0 * b + 2.0 * c + d))
        .collect())
}

/// Deterministic integration with discrete shock events applied on crossing.
pub fn run_simulation(model: &SolverModel<'_>, cfg: &SimConfig) -> Result<TimeSeries, ChronoError> {
    validate_config(cfg)?;
    let steps = cfg.steps.max(1);
    let mut times = Vec::with_capacity(steps + 1);
    let mut cols: Vec<Vec<f64>> = model.initials.iter().map(|v| vec![*v]).collect();
    times.push(0.0);
    let mut y = model.initials.clone();
    let mut t = 0.0;
    for _ in 0..steps {
        y = rk4_step(model, &y, t, cfg.dt)?;
        for (stock_id, shock_time, mag) in model.shocks.iter() {
            if t <= *shock_time && *shock_time < t + cfg.dt {
                if let Some(pos) = model.stock_ids.iter().position(|s| s == stock_id) {
                    y[pos] += *mag;
                }
            }
        }
        t += cfg.dt;
        times.push(t);
        for (col, val) in cols.iter_mut().zip(&y) {
            col.push(*val);
        }
    }
    Ok(TimeSeries {
        times,
        series: model
            .stock_ids
            .iter()
            .zip(cols)
            .map(|(id, values)| StockSeries {
                stock: id.clone(),
                values,
            })
            .collect(),
        steps,
        dt: cfg.dt,
    })
}

pub fn validate_config(cfg: &SimConfig) -> Result<(), ChronoError> {
    if cfg.dt <= 0.0 || !cfg.dt.is_finite() {
        return Err(ChronoError::Validation(
            "dt must be a positive finite number".to_string(),
        ));
    }
    if cfg.steps == 0 || cfg.steps > 100_000 {
        return Err(ChronoError::Validation(
            "steps must be between 1 and 100000".to_string(),
        ));
    }
    if !cfg.shock_std.is_finite() || cfg.shock_std < 0.0 || cfg.shock_std > 10.0 {
        return Err(ChronoError::Validation(
            "shock_std must be between 0 and 10".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decay_model() -> (Vec<String>, Vec<f64>, Vec<Flow>, HashMap<String, f64>) {
        (
            vec!["s".to_string()],
            vec![100.0],
            vec![Flow {
                id: "f".to_string(),
                label: "decay".to_string(),
                from_stock: Some("s".to_string()),
                to_stock: None,
                rate_expr: "k * s".to_string(),
                x: 0.0,
                y: 0.0,
            }],
            HashMap::from([("k".to_string(), 0.5)]),
        )
    }

    #[test]
    fn rk4_tracks_analytic_exponential_decay() {
        let (ids, initials, flows_vec, params) = decay_model();
        let model = build_solver_model(&ids, &initials, &flows_vec, &params, &[]);
        let cfg = SimConfig {
            dt: 0.01,
            steps: 200,
            shock_std: 0.0,
            seed: 1,
        };
        let ts = run_simulation(&model, &cfg);
        assert!(ts.is_ok());
        if let Ok(trace) = ts {
            // Analytic: 100 * exp(-0.5 * 2.0)
            let analytic = 100.0 * (-1.0_f64).exp();
            let end = trace.end_state("s");
            assert!(end.is_some());
            if let Some(v) = end {
                let rel = ((v - analytic) / analytic).abs();
                assert!(rel < 1e-4);
            }
        }
    }

    #[test]
    fn invalid_configs_are_rejected() {
        let bad_dt = SimConfig {
            dt: 0.0,
            steps: 10,
            shock_std: 0.0,
            seed: 1,
        };
        assert!(validate_config(&bad_dt).is_err());
        let bad_steps = SimConfig {
            dt: 0.05,
            steps: 0,
            shock_std: 0.0,
            seed: 1,
        };
        assert!(validate_config(&bad_steps).is_err());
    }
}
