//! Domain models for ChronoCraft system-dynamics graphs.
//!
//! A model is a stocks-and-flows graph: stocks accumulate quantities, flows
//! move quantity between stocks (or across the model boundary), feedback links
//! record which stocks inform each flow rate, and lumped parameters tune rates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stock {
    pub id: String,
    pub label: String,
    pub initial: f64,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub id: String,
    pub label: String,
    pub from_stock: Option<String>,
    pub to_stock: Option<String>,
    pub rate_expr: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub from_node: String,
    pub to_flow: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub value: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShockEvent {
    pub id: String,
    pub stock_id: String,
    pub time: f64,
    pub magnitude: f64,
}

/// Integration and Monte Carlo configuration shared by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub dt: f64,
    pub steps: usize,
    pub shock_std: f64,
    pub seed: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            dt: 0.05,
            steps: 200,
            shock_std: 0.05,
            seed: 42,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPreset {
    Population,
    Inventory,
    EpidemicSir,
}

impl ModelPreset {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "population" | "pop" | "logistic" => Some(Self::Population),
            "inventory" | "stock" => Some(Self::Inventory),
            "epidemic_sir" | "epidemicsir" | "sir" => Some(Self::EpidemicSir),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Population => "population",
            Self::Inventory => "inventory",
            Self::EpidemicSir => "epidemic_sir",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSeries {
    pub stock: String,
    pub values: Vec<f64>,
}

/// Deterministic simulation trace: one value per step per stock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub times: Vec<f64>,
    pub series: Vec<StockSeries>,
    pub steps: usize,
    pub dt: f64,
}

impl TimeSeries {
    pub fn end_state(&self, stock: &str) -> Option<f64> {
        self.series
            .iter()
            .find(|s| s.stock == stock)
            .and_then(|s| s.values.last().copied())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileBand {
    pub stock: String,
    pub p10: Vec<f64>,
    pub p50: Vec<f64>,
    pub p90: Vec<f64>,
}

/// Monte Carlo outcome: percentile bands over shared time points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResult {
    pub runs: usize,
    pub seed: u64,
    pub shock_std: f64,
    pub times: Vec<f64>,
    pub bands: Vec<PercentileBand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockDiff {
    pub stock: String,
    pub baseline: f64,
    pub variant: f64,
    pub delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDiff {
    pub baseline: String,
    pub variant: String,
    pub end: Vec<StockDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitivityResult {
    pub param: String,
    pub target: String,
    pub values: Vec<f64>,
    pub end_states: Vec<f64>,
}
