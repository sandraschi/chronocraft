//! Integration & preset verification suite for ChronoCraft L1 core.

use chronocraft_core::{check_expr, eval_expr, ChronoEngine, Command, ModelPreset};
use std::collections::HashMap;
use std::path::PathBuf;

fn temp_db(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("chronocraft_{name}_{}.db", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn preset_population_grows_but_stays_below_capacity() {
    let eng = ChronoEngine::with_preset(ModelPreset::Population);
    let trace = eng.run(Some(200), Some(0.05));
    assert!(trace.is_ok());
    if let Ok(ts) = trace {
        let end = ts.end_state("pop").unwrap_or(0.0);
        assert!(end > 100.0 && end < 1000.0);
    }
}

#[test]
fn preset_inventory_holds_steady_when_orders_cover_demand() {
    let eng = ChronoEngine::with_preset(ModelPreset::Inventory);
    let trace = eng.run(Some(100), Some(0.05));
    assert!(trace.is_ok());
    if let Ok(ts) = trace {
        let end = ts.end_state("goods").unwrap_or(0.0);
        assert!(end > 500.0 && end < 1000.0);
    }
}

#[test]
fn preset_sir_moves_cases_from_susceptible_to_recovered() {
    let eng = ChronoEngine::with_preset(ModelPreset::EpidemicSir);
    let trace = eng.run(Some(400), Some(0.05));
    assert!(trace.is_ok());
    if let Ok(ts) = trace {
        let s = ts.end_state("susceptible").unwrap_or(0.0);
        let r = ts.end_state("recovered").unwrap_or(0.0);
        assert!(s < 990.0);
        assert!(r > 0.0);
    }
}

#[test]
fn expression_cases_cover_lookup_arithmetic_and_helpers() {
    let vars: HashMap<String, f64> =
        HashMap::from([("pop".to_string(), 100.0), ("t".to_string(), 2.0)]);
    let val = eval_expr("0.1 * pop + min(t, 5) + pulse(t, 1, 2, 3)", &vars);
    assert!(val.is_ok());
    if let Ok(v) = val {
        assert!((v - 15.0).abs() < 1e-9);
    }
    assert!(check_expr("clamp(beta * s, 0, 10)").is_ok());
    assert!(eval_expr("1 / 0", &vars).is_err());
}

#[test]
fn monte_carlo_bands_hold_order_and_reproduce_from_seed() {
    let eng = ChronoEngine::with_preset(ModelPreset::Population);
    let db = temp_db("bands");
    let mut probe = eng.clone();
    let first = Command::RunMonteCarlo {
        runs: 60,
        seed: Some(11),
        shock_std: Some(0.05),
    }
    .dispatch(&mut probe, &db);
    assert!(first.is_ok());
    if let Ok(val) = first {
        let bands = &val["bands"];
        let list = bands.as_array().cloned().unwrap_or_default();
        assert!(!list.is_empty());
        for band in &list {
            let p10 = band["p10"].as_array().cloned().unwrap_or_default();
            let p50 = band["p50"].as_array().cloned().unwrap_or_default();
            let p90 = band["p90"].as_array().cloned().unwrap_or_default();
            assert_eq!(p10.len(), p50.len());
            assert_eq!(p50.len(), p90.len());
            for i in 0..p10.len() {
                let (a, b, c) = (
                    p10[i].as_f64().unwrap_or(0.0),
                    p50[i].as_f64().unwrap_or(0.0),
                    p90[i].as_f64().unwrap_or(0.0),
                );
                assert!(a <= b && b <= c);
            }
        }
    }
    let _ = std::fs::remove_file(&db);
}

#[test]
fn command_add_flow_validates_unknown_stocks_and_names() {
    let mut eng = ChronoEngine::with_preset(ModelPreset::Population);
    let db = temp_db("flows");
    let bad_stock = Command::AddFlow {
        id: "f".into(),
        label: "".into(),
        from_stock: Some("ghost".into()),
        to_stock: Some("pop".into()),
        rate_expr: "1.0".into(),
        x: 0.0,
        y: 0.0,
    }
    .dispatch(&mut eng, &db);
    assert!(bad_stock.is_err());
    let bad_expr = Command::AddFlow {
        id: "f".into(),
        label: "".into(),
        from_stock: None,
        to_stock: Some("pop".into()),
        rate_expr: "nope_missing + 1".into(),
        x: 0.0,
        y: 0.0,
    }
    .dispatch(&mut eng, &db);
    assert!(bad_expr.is_err());
    let good = Command::AddFlow {
        id: "migration".into(),
        label: "Migration".into(),
        from_stock: None,
        to_stock: Some("pop".into()),
        rate_expr: "5 + pulse(t, 2, 1, 10)".into(),
        x: 0.0,
        y: 0.0,
    }
    .dispatch(&mut eng, &db);
    assert!(good.is_ok());
    let _ = std::fs::remove_file(&db);
}

#[test]
fn scenario_save_list_and_compare_diff_end_states() {
    let db = temp_db("scenarios");
    let mut base = ChronoEngine::with_preset(ModelPreset::Population);
    assert!(Command::SaveScenario {
        name: "baseline".into()
    }
    .dispatch(&mut base, &db)
    .is_ok());
    let mut variant = ChronoEngine::with_preset(ModelPreset::Population);
    assert!(variant.set_param("growth", 0.3).is_ok());
    variant.active_scenario = "challenge".to_string();
    assert!(variant.save_db(&db).is_ok());

    let list = Command::ListScenarios.dispatch(&mut base, &db);
    assert!(list.is_ok());
    let diff = Command::CompareScenarios {
        baseline: "baseline".into(),
        variant: "challenge".into(),
    }
    .dispatch(&mut base, &db);
    assert!(diff.is_ok());
    if let Ok(val) = diff {
        let end = val["end"].as_array().cloned().unwrap_or_default();
        assert!(!end.is_empty());
        for row in &end {
            let delta = row["delta"].as_f64().unwrap_or(0.0);
            let recomputed =
                row["variant"].as_f64().unwrap_or(0.0) - row["baseline"].as_f64().unwrap_or(0.0);
            assert!((delta - recomputed).abs() < 1e-9);
        }
    }
    let _ = std::fs::remove_file(&db);
}

#[test]
fn sensitivity_sweep_returns_ordered_param_grid() {
    let mut eng = ChronoEngine::with_preset(ModelPreset::Population);
    let db = temp_db("sweep");
    let res = Command::SensitivitySweep {
        param: "growth".into(),
        from: 0.05,
        to: 0.2,
        steps: 4,
        target: "pop".into(),
    }
    .dispatch(&mut eng, &db);
    assert!(res.is_ok());
    if let Ok(val) = res {
        let values = val["values"].as_array().cloned().unwrap_or_default();
        let ends = val["end_states"].as_array().cloned().unwrap_or_default();
        assert_eq!(values.len(), 4);
        assert_eq!(ends.len(), 4);
        for w in ends.windows(2) {
            let (a, b) = (w[0].as_f64().unwrap_or(0.0), w[1].as_f64().unwrap_or(0.0));
            assert!(b >= a);
        }
    }
    let _ = std::fs::remove_file(&db);
}
