//! ChronoCraft L2 CLI & MCP `2026-07-28` stdio server.
#![forbid(unsafe_code)]

use chronocraft_core::{default_db_path, ChronoEngine, Command, APP_VERSION};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "chronocraft-mcp",
    version,
    about = "ChronoCraft System-Dynamics and Monte Carlo Scenario Simulator"
)]
pub struct Cli {
    /// Emit structured JSON output
    #[arg(long, global = true, default_value_t = false)]
    pub json: bool,

    /// Optional SQLite database file path override
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Launch MCP 2026-07-28 JSON-RPC server on stdio
    #[arg(long, global = true, default_value_t = false)]
    pub mcp: bool,

    #[command(subcommand)]
    pub subcommand: Option<CliSub>,
}

#[derive(Debug, Subcommand)]
pub enum CliSub {
    /// Run MCP 2026-07-28 JSON-RPC server on stdio
    Mcp,
    /// Show ChronoCraft engine status
    Status,
    /// Show full model state (stocks, flows, links, parameters)
    Summary,
    /// Load a built-in model preset (population, inventory, epidemic_sir)
    LoadPreset {
        #[arg(long, default_value = "population")]
        preset: String,
    },
    /// Add a stock to the active model
    AddStock {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "")]
        label: String,
        #[arg(long, default_value_t = 100.0)]
        initial: f64,
        #[arg(long, default_value_t = 320.0)]
        x: f32,
        #[arg(long, default_value_t = 140.0)]
        y: f32,
    },
    /// Add a flow to the active model
    AddFlow {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "")]
        label: String,
        #[arg(long)]
        from_stock: Option<String>,
        #[arg(long)]
        to_stock: Option<String>,
        #[arg(long)]
        rate_expr: String,
        #[arg(long, default_value_t = 200.0)]
        x: f32,
        #[arg(long, default_value_t = 140.0)]
        y: f32,
    },
    /// Link a stock as feedback into a flow rate
    AddLink {
        #[arg(long)]
        from_node: String,
        #[arg(long)]
        to_flow: String,
    },
    /// Set a lumped parameter value
    SetParam {
        #[arg(long)]
        name: String,
        #[arg(long)]
        value: f64,
    },
    /// Run the deterministic RK4 simulation
    RunSimulation {
        #[arg(long)]
        steps: Option<usize>,
        #[arg(long)]
        dt: Option<f64>,
    },
    /// Run the seeded Monte Carlo engine (10..10000 runs)
    RunMonteCarlo {
        #[arg(long, default_value_t = 500)]
        runs: usize,
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        shock_std: Option<f64>,
    },
    /// Save the active model as a named scenario
    SaveScenario {
        #[arg(long)]
        name: String,
    },
    /// List persisted scenarios
    ListScenarios,
    /// Compare baseline vs variant end-state stocks
    Compare {
        #[arg(long)]
        baseline: String,
        #[arg(long)]
        variant: String,
    },
    /// Sweep one parameter and record a target stock end state
    Sensitivity {
        #[arg(long)]
        param: String,
        #[arg(long)]
        from: f64,
        #[arg(long)]
        to: f64,
        #[arg(long, default_value_t = 5)]
        steps: usize,
        #[arg(long, default_value = "pop")]
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let db_path = cli.db.unwrap_or_else(default_db_path);
    let mut engine = ChronoEngine::open_or_seed(&db_path).unwrap_or_default();

    if cli.mcp || matches!(cli.subcommand, Some(CliSub::Mcp)) {
        run_mcp_stdio(&mut engine, &db_path);
        return;
    }

    let cmd = match cli.subcommand.unwrap_or(CliSub::Status) {
        CliSub::Mcp | CliSub::Status => Command::Status,
        CliSub::Summary => Command::Summary,
        CliSub::LoadPreset { preset } => Command::LoadPreset { preset },
        CliSub::AddStock {
            id,
            label,
            initial,
            x,
            y,
        } => Command::AddStock {
            id,
            label,
            initial,
            x,
            y,
        },
        CliSub::AddFlow {
            id,
            label,
            from_stock,
            to_stock,
            rate_expr,
            x,
            y,
        } => Command::AddFlow {
            id,
            label,
            from_stock,
            to_stock,
            rate_expr,
            x,
            y,
        },
        CliSub::AddLink { from_node, to_flow } => Command::AddLink { from_node, to_flow },
        CliSub::SetParam { name, value } => Command::SetParam { name, value },
        CliSub::RunSimulation { steps, dt } => Command::RunSimulation { steps, dt },
        CliSub::RunMonteCarlo {
            runs,
            seed,
            shock_std,
        } => Command::RunMonteCarlo {
            runs,
            seed,
            shock_std,
        },
        CliSub::SaveScenario { name } => Command::SaveScenario { name },
        CliSub::ListScenarios => Command::ListScenarios,
        CliSub::Compare { baseline, variant } => Command::CompareScenarios { baseline, variant },
        CliSub::Sensitivity {
            param,
            from,
            to,
            steps,
            target,
        } => Command::SensitivitySweep {
            param,
            from,
            to,
            steps,
            target,
        },
    };

    match cmd.dispatch(&mut engine, &db_path) {
        Ok(val) => {
            let _ = engine.save_db(&db_path);
            let out = serde_json::to_string_pretty(&val).unwrap_or_else(|_| "{}".to_string());
            println!("{out}");
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

pub fn handle_mcp_request(req: &Value, engine: &mut ChronoEngine, db_path: &Path) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned();

    if method.starts_with("notifications/") {
        return None;
    }
    let req_id = id?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2026-07-28",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "chronocraft-mcp",
                    "version": APP_VERSION
                }
            }
        })),
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": mcp_tools_schema()
            }
        })),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let cmd_res = match name {
                "chrono_status" => Command::Status.dispatch(engine, db_path),
                "chrono_summary" => Command::Summary.dispatch(engine, db_path),
                "load_preset" => {
                    let preset = args
                        .get("preset")
                        .and_then(Value::as_str)
                        .unwrap_or("population")
                        .to_string();
                    Command::LoadPreset { preset }.dispatch(engine, db_path)
                }
                "add_stock" => {
                    let id = args
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let label = args
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let initial = args.get("initial").and_then(Value::as_f64).unwrap_or(100.0);
                    let x = args.get("x").and_then(Value::as_f64).unwrap_or(320.0) as f32;
                    let y = args.get("y").and_then(Value::as_f64).unwrap_or(140.0) as f32;
                    Command::AddStock {
                        id,
                        label,
                        initial,
                        x,
                        y,
                    }
                    .dispatch(engine, db_path)
                }
                "add_flow" => {
                    let id = args
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let label = args
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let from_stock = args
                        .get("from_stock")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let to_stock = args
                        .get("to_stock")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let rate_expr = args
                        .get("rate_expr")
                        .and_then(Value::as_str)
                        .unwrap_or("1.0")
                        .to_string();
                    let x = args.get("x").and_then(Value::as_f64).unwrap_or(200.0) as f32;
                    let y = args.get("y").and_then(Value::as_f64).unwrap_or(140.0) as f32;
                    Command::AddFlow {
                        id,
                        label,
                        from_stock,
                        to_stock,
                        rate_expr,
                        x,
                        y,
                    }
                    .dispatch(engine, db_path)
                }
                "add_link" => {
                    let from_node = args
                        .get("from_node")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let to_flow = args
                        .get("to_flow")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Command::AddLink { from_node, to_flow }.dispatch(engine, db_path)
                }
                "set_param" => {
                    let pname = args
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let value = args.get("value").and_then(Value::as_f64).unwrap_or(0.0);
                    Command::SetParam { name: pname, value }.dispatch(engine, db_path)
                }
                "run_simulation" => {
                    let steps = args
                        .get("steps")
                        .and_then(Value::as_u64)
                        .map(|s| s as usize);
                    let dt = args.get("dt").and_then(Value::as_f64);
                    Command::RunSimulation { steps, dt }.dispatch(engine, db_path)
                }
                "run_monte_carlo" => {
                    let runs = args.get("runs").and_then(Value::as_u64).unwrap_or(500) as usize;
                    let seed = args.get("seed").and_then(Value::as_u64);
                    let shock_std = args.get("shock_std").and_then(Value::as_f64);
                    Command::RunMonteCarlo {
                        runs,
                        seed,
                        shock_std,
                    }
                    .dispatch(engine, db_path)
                }
                "save_scenario" => {
                    let sname = args
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Command::SaveScenario { name: sname }.dispatch(engine, db_path)
                }
                "list_scenarios" => Command::ListScenarios.dispatch(engine, db_path),
                "compare_scenarios" => {
                    let baseline = args
                        .get("baseline")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let variant = args
                        .get("variant")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Command::CompareScenarios { baseline, variant }.dispatch(engine, db_path)
                }
                "sensitivity_sweep" => {
                    let param = args
                        .get("param")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let from = args.get("from").and_then(Value::as_f64).unwrap_or(0.0);
                    let to = args.get("to").and_then(Value::as_f64).unwrap_or(1.0);
                    let steps = args.get("steps").and_then(Value::as_u64).unwrap_or(5) as usize;
                    let target = args
                        .get("target")
                        .and_then(Value::as_str)
                        .unwrap_or("pop")
                        .to_string();
                    Command::SensitivitySweep {
                        param,
                        from,
                        to,
                        steps,
                        target,
                    }
                    .dispatch(engine, db_path)
                }
                other => Err(chronocraft_core::ChronoError::Validation(format!(
                    "Unknown tool '{other}'"
                ))),
            };

            match cmd_res {
                Ok(val) => {
                    let _ = engine.save_db(db_path);
                    let text = serde_json::to_string_pretty(&val).unwrap_or_default();
                    Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [{ "type": "text", "text": text }],
                            "isError": false
                        }
                    }))
                }
                Err(e) => Some(json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [{ "type": "text", "text": e.to_string() }],
                        "isError": true
                    }
                })),
            }
        }
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        })),
    }
}

fn mcp_tools_schema() -> Value {
    json!([
        { "name": "chrono_status", "description": "Get ChronoCraft engine status, active scenario, and stock/flow/link/parameter counts.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "chrono_summary", "description": "Return the full active model state (stocks, flows, links, parameters, config).", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "load_preset", "description": "Load a model preset (population, inventory, epidemic_sir).", "inputSchema": { "type": "object", "properties": { "preset": { "type": "string" } }, "required": ["preset"] } },
        { "name": "add_stock", "description": "Add a stock (id, label, initial value, x, y) to the active model.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "label": { "type": "string" }, "initial": { "type": "number" }, "x": { "type": "number" }, "y": { "type": "number" } }, "required": ["id"] } },
        { "name": "add_flow", "description": "Add a flow with a rate expression between stocks (or across the model boundary).", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "label": { "type": "string" }, "from_stock": { "type": "string" }, "to_stock": { "type": "string" }, "rate_expr": { "type": "string" }, "x": { "type": "number" }, "y": { "type": "number" } }, "required": ["id", "rate_expr"] } },
        { "name": "add_link", "description": "Record a stock as feedback for a flow rate.", "inputSchema": { "type": "object", "properties": { "from_node": { "type": "string" }, "to_flow": { "type": "string" } }, "required": ["from_node", "to_flow"] } },
        { "name": "set_param", "description": "Set a lumped parameter value (created when missing).", "inputSchema": { "type": "object", "properties": { "name": { "type": "string" }, "value": { "type": "number" } }, "required": ["name", "value"] } },
        { "name": "run_simulation", "description": "Run the deterministic RK4 simulation and return per-stock traces.", "inputSchema": { "type": "object", "properties": { "steps": { "type": "integer" }, "dt": { "type": "integer" } } } },
        { "name": "run_monte_carlo", "description": "Run the seeded Monte Carlo engine and return p10/p50/p90 bands.", "inputSchema": { "type": "object", "properties": { "runs": { "type": "integer" }, "seed": { "type": "integer" }, "shock_std": { "type": "number" } } } },
        { "name": "save_scenario", "description": "Persist the active model as a named scenario.", "inputSchema": { "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] } },
        { "name": "list_scenarios", "description": "List persisted scenario names and the active scenario.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "compare_scenarios", "description": "Compare baseline vs variant end-state stocks.", "inputSchema": { "type": "object", "properties": { "baseline": { "type": "string" }, "variant": { "type": "string" } }, "required": ["baseline", "variant"] } },
        { "name": "sensitivity_sweep", "description": "Sweep one parameter across a range and record a target stock end state.", "inputSchema": { "type": "object", "properties": { "param": { "type": "string" }, "from": { "type": "number" }, "to": { "type": "number" }, "steps": { "type": "integer" }, "target": { "type": "string" } }, "required": ["param", "from", "to", "target"] } }
    ])
}

fn run_mcp_stdio(engine: &mut ChronoEngine, db_path: &Path) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(raw) = line else { break };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(resp) = handle_mcp_request(&req, engine, db_path) {
            if let Ok(s) = serde_json::to_string(&resp) {
                let _ = writeln!(stdout, "{s}");
                let _ = stdout.flush();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chronocraft_mcp_test_{}.db", std::process::id()));
        p
    }

    #[test]
    fn mcp_initialize_and_tools_list_return_13_tools() {
        let mut eng = ChronoEngine::default();
        let db = temp_db();
        let init = handle_mcp_request(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            &mut eng,
            &db,
        );
        assert!(init.is_some());

        let list = handle_mcp_request(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        let tools = list["result"]["tools"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        assert_eq!(tools, 13);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn mcp_tools_call_executes_simulation_monte_carlo_and_compare() {
        let mut eng = ChronoEngine::default();
        let db = temp_db();

        let sim = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": { "name": "run_simulation", "arguments": { "steps": 50 } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(sim["result"]["isError"], false);

        let mc = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": { "name": "run_monte_carlo", "arguments": { "runs": 20, "seed": 5 } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(mc["result"]["isError"], false);

        let save_a = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": { "name": "save_scenario", "arguments": { "name": "a" } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(save_a["result"]["isError"], false);

        let set = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": { "name": "set_param", "arguments": { "name": "growth", "value": 0.3 } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(set["result"]["isError"], false);

        let save_b = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": { "name": "save_scenario", "arguments": { "name": "b" } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(save_b["result"]["isError"], false);

        let cmp = handle_mcp_request(
            &json!({
                "jsonrpc": "2.0",
                "id": 8,
                "method": "tools/call",
                "params": { "name": "compare_scenarios", "arguments": { "baseline": "a", "variant": "b" } }
            }),
            &mut eng,
            &db,
        )
        .unwrap_or_default();
        assert_eq!(cmp["result"]["isError"], false);
        let _ = std::fs::remove_file(&db);
    }
}
