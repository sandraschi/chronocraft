//! Seeded Monte Carlo engine with p10/p50/p90 percentile bands.
//!
//! The generator is a self-contained xorshift64* stream with a Box-Muller
//! transform, so runs are reproducible from `seed + run_index` with no extra
//! dependencies.

use crate::error::ChronoError;
use crate::model::{MonteCarloResult, PercentileBand, SimConfig};
use crate::sim::{rk4_step, validate_config, SolverModel};

/// Deterministic 64-bit generator (xorshift64*).
pub struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    pub fn new(seed: u64) -> Self {
        let s = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state: s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_unit(&mut self) -> f64 {
        // 53-bit resolution uniform in [0, 1).
        let bits = self.next_u64() >> 11;
        (bits as f64) / (18446744073709551616.0)
    }

    /// Standard-normal sample via Box-Muller.
    pub fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_unit().max(1e-12);
        let u2 = self.next_unit();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

/// Nearest-rank percentile with linear interpolation on sorted samples.
pub fn percentile(sorted: &[f64], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0]);
    }
    let rank = q.clamp(0.0, 1.0) * ((sorted.len() - 1) as f64);
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - (lo as f64);
    let a = sorted.get(lo).copied().unwrap_or(0.0);
    let b = sorted.get(hi).copied().unwrap_or(a);
    Some(a + frac * (b - a))
}

/// Run the model `runs` times with random shocks and reduce to bands.
pub fn run_monte_carlo(
    model: &SolverModel<'_>,
    cfg: &SimConfig,
    runs: usize,
    seed: u64,
    shock_std: f64,
) -> Result<MonteCarloResult, ChronoError> {
    validate_config(cfg)?;
    if !(10..=10_000).contains(&runs) {
        return Err(ChronoError::Validation(
            "runs must be between 10 and 10000".to_string(),
        ));
    }
    if !shock_std.is_finite() || shock_std < 0.0 || shock_std > 10.0 {
        return Err(ChronoError::Validation(
            "shock_std must be between 0 and 10".to_string(),
        ));
    }
    let steps = cfg.steps.max(1);
    let mut times = Vec::with_capacity(steps + 1);
    let mut t = 0.0;
    times.push(t);
    for _ in 0..steps {
        t += cfg.dt;
        times.push(t);
    }

    let mut acc: Vec<Vec<Vec<f64>>> = Vec::with_capacity(runs);
    for run in 0..runs {
        let mut rng = XorShift64::new(seed.wrapping_add(run as u64 + 1));
        let mut jittered = model.params.clone();
        for v in jittered.values_mut() {
            *v *= 1.0 + shock_std * rng.next_gaussian();
        }
        let noisy = SolverModel {
            stock_ids: model.stock_ids.clone(),
            initials: model.initials.clone(),
            flows: model.flows,
            params: jittered,
            shocks: model.shocks.clone(),
        };
        let mut y: Vec<f64> = noisy
            .initials
            .iter()
            .map(|v| v * (1.0 + shock_std * rng.next_gaussian()))
            .collect();
        let mut trace: Vec<Vec<f64>> = y.iter().map(|v| vec![*v]).collect();
        let mut tt = 0.0;
        for _ in 0..steps {
            let stepped = rk4_step(&noisy, &y, tt, cfg.dt)?;
            y = stepped
                .into_iter()
                .map(|v| v + shock_std * cfg.dt.sqrt() * rng.next_gaussian())
                .collect::<Vec<f64>>();
            tt += cfg.dt;
            for (col, val) in trace.iter_mut().zip(&y) {
                col.push(*val);
            }
        }
        acc.push(trace);
    }

    let mut bands = Vec::with_capacity(model.stock_ids.len());
    for (sidx, stock) in model.stock_ids.iter().enumerate() {
        let mut p10 = Vec::with_capacity(steps + 1);
        let mut p50 = Vec::with_capacity(steps + 1);
        let mut p90 = Vec::with_capacity(steps + 1);
        for step in 0..=steps {
            let mut samples: Vec<f64> = acc.iter().map(|tr| tr[sidx][step]).collect();
            samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            p10.push(percentile(&samples, 0.10).unwrap_or(0.0));
            p50.push(percentile(&samples, 0.50).unwrap_or(0.0));
            p90.push(percentile(&samples, 0.90).unwrap_or(0.0));
        }
        bands.push(PercentileBand {
            stock: stock.clone(),
            p10,
            p50,
            p90,
        });
    }

    Ok(MonteCarloResult {
        runs,
        seed,
        shock_std,
        times,
        bands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Flow;
    use std::collections::HashMap;

    fn growth_model() -> (Vec<String>, Vec<f64>, Vec<Flow>, HashMap<String, f64>) {
        (
            vec!["pop".to_string()],
            vec![100.0],
            vec![Flow {
                id: "births".to_string(),
                label: "births".to_string(),
                from_stock: None,
                to_stock: Some("pop".to_string()),
                rate_expr: "growth * pop".to_string(),
                x: 0.0,
                y: 0.0,
            }],
            HashMap::from([("growth".to_string(), 0.05)]),
        )
    }

    #[test]
    fn bands_stay_ordered_at_every_step() {
        let (ids, initials, flows_vec, params) = growth_model();
        let model = crate::sim::build_solver_model(&ids, &initials, &flows_vec, &params, &[]);
        let cfg = SimConfig {
            dt: 0.05,
            steps: 60,
            shock_std: 0.0,
            seed: 7,
        };
        let res = run_monte_carlo(&model, &cfg, 50, 7, 0.08);
        assert!(res.is_ok());
        if let Ok(mc) = res {
            assert_eq!(mc.runs, 50);
            for band in &mc.bands {
                assert_eq!(band.p10.len(), mc.times.len());
                for i in 0..band.p10.len() {
                    assert!(band.p10[i] <= band.p50[i]);
                    assert!(band.p50[i] <= band.p90[i]);
                }
            }
        }
    }

    #[test]
    fn invalid_run_counts_are_rejected() {
        let (ids, initials, flows_vec, params) = growth_model();
        let model = crate::sim::build_solver_model(&ids, &initials, &flows_vec, &params, &[]);
        let cfg = SimConfig::default();
        assert!(run_monte_carlo(&model, &cfg, 5, 1, 0.05).is_err());
        assert!(run_monte_carlo(&model, &cfg, 50_000, 1, 0.05).is_err());
    }

    #[test]
    fn seeded_runs_reproduce_identical_bands() {
        let (ids, initials, flows_vec, params) = growth_model();
        let model = crate::sim::build_solver_model(&ids, &initials, &flows_vec, &params, &[]);
        let cfg = SimConfig {
            dt: 0.05,
            steps: 20,
            shock_std: 0.0,
            seed: 3,
        };
        let a = run_monte_carlo(&model, &cfg, 30, 99, 0.1);
        let b = run_monte_carlo(&model, &cfg, 30, 99, 0.1);
        assert!(a.is_ok() && b.is_ok());
        if let (Ok(ra), Ok(rb)) = (a, b) {
            assert_eq!(ra.bands.len(), rb.bands.len());
            for (ba, bb) in ra.bands.iter().zip(rb.bands.iter()) {
                assert_eq!(ba.p50.len(), bb.p50.len());
                for (x, y) in ba.p50.iter().zip(bb.p50.iter()) {
                    assert!((x - y).abs() < 1e-12);
                }
            }
        }
    }
}
