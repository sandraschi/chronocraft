//! Comprehensive 6-Tab In-App Documentation for ChronoCraft.

use crate::app::{ChronoCraftApp, HelpTab};
use chronocraft_core::APP_VERSION;
use eframe::egui;

const HELP_MD: &str = include_str!("../../../HELP.md");

pub fn render_help_tab_content(app: &ChronoCraftApp, ui: &mut egui::Ui) {
    match app.help_tab {
        HelpTab::GetStarted => {
            ui.label(HELP_MD);
        }
        HelpTab::CanvasStocks => {
            ui.heading("Workbench 1: Stocks, Flows & Feedback Links");
            ui.label(
                "Stocks accumulate quantities over time. Flows move quantity into a stock, out of a stock, or between two stocks. A flow with no source draws from outside the model; a flow with no target drains outside the model.",
            );
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("1. Canvas gestures");
                ui.label("- Drag any stock or flow node to reposition it on the canvas.");
                ui.label("- Use Presets & Model Builder to add stocks, flows, and feedback links.");
                ui.label("- Links record which stocks inform each flow rate; rates can read any stock id, any parameter name, and model time `t`.");
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("2. Built-in model presets");
                ui.label("- population: logistic growth toward carrying capacity with proportional loss.");
                ui.label("- inventory: goods on hand with inbound orders and customer demand.");
                ui.label("- epidemic_sir: susceptible/infected/recovered feedback loop with infection and recovery rates.");
            });
        }
        HelpTab::SlidersParams => {
            ui.heading("Workbench 2: Lumped Parameters & Solver Settings");
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.strong("1. Parameter sliders (Subpage 1)");
                ui.label("Every lumped parameter carries a min/max range. Move a slider, then press Apply Parameters: the studio sends one SetParam command per changed parameter, so agents can replay the same change.");
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("2. Solver settings (Subpage 2)");
                ui.label("Step size dt, step count, shock width, and RNG seed are local run settings. Smaller dt resolves fast feedback loops; keep steps below 100000. Every run passes through the same RK4 and Monte Carlo commands the CLI and MCP tools call.");
            });
        }
        HelpTab::BandsCompare => {
            ui.heading("Workbench 3 & 4: Fan Chart, Scenarios & Sensitivity");
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.strong("1. Deterministic trace");
                ui.label("Classic fourth-order Runge-Kutta integration of every stock. Compare the curve against the analytic decay test tolerance when you change dt.");
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("2. Percentile bands");
                ui.label("The seeded engine replays the model up to 10,000 times with random shocks and reduces runs to p10/p50/p90 bands. The shaded band spans p10..p90 and the bright line tracks the p50 median. Same seed plus same model always paints the same band.");
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("3. Scenarios, compare & sensitivity");
                ui.label("Save the active model under a name, list persisted scenarios, then compare a baseline against a variant: the studio diffs every end-state stock (variant minus baseline). A sensitivity sweep moves one parameter across a from/to grid (2..21 points) and records a target stock end state per point.");
            });
        }
        HelpTab::CliMcp => {
            ui.heading("L2 CLI Subcommands & 13 MCP 2026-07-28 Tools Reference");
            ui.label("Every GUI button, CLI command, and MCP tool call routes through L1 'Command::dispatch'.");
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("1. Complete 13 MCP Tools Table");
                egui::Grid::new("help_chrono_mcp_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("MCP Tool Name");
                        ui.strong("CLI Subcommand");
                        ui.strong("Description");
                        ui.end_row();
                        for (tool, cli, desc) in [
                            (
                                "chrono_status",
                                "status",
                                "Engine version, active scenario, and model counts",
                            ),
                            (
                                "chrono_summary",
                                "summary",
                                "Full model state (stocks, flows, links, parameters)",
                            ),
                            (
                                "load_preset",
                                "load-preset",
                                "Load population, inventory, or epidemic_sir",
                            ),
                            (
                                "add_stock",
                                "add-stock",
                                "Add a stock node (id, label, initial, x, y)",
                            ),
                            ("add_flow", "add-flow", "Add a flow with a rate expression"),
                            ("add_link", "add-link", "Record stock feedback into a flow"),
                            ("set_param", "set-param", "Set a lumped parameter value"),
                            (
                                "run_simulation",
                                "run-simulation",
                                "Run deterministic RK4 integration",
                            ),
                            (
                                "run_monte_carlo",
                                "run-monte-carlo",
                                "Run seeded Monte Carlo p10/p50/p90 bands",
                            ),
                            (
                                "save_scenario",
                                "save-scenario",
                                "Persist the active model under a name",
                            ),
                            (
                                "list_scenarios",
                                "list-scenarios",
                                "List persisted scenario names",
                            ),
                            (
                                "compare_scenarios",
                                "compare",
                                "Diff baseline vs variant end states",
                            ),
                            (
                                "sensitivity_sweep",
                                "sensitivity",
                                "Sweep one parameter, record target end state",
                            ),
                        ] {
                            ui.monospace(tool);
                            ui.monospace(cli);
                            ui.label(desc);
                            ui.end_row();
                        }
                    });
            });
        }
        HelpTab::TechnicalInfo => {
            ui.heading("Technical Architecture, SQLite Schema & Solver Notes");
            ui.add_space(4.0);
            ui.group(|ui| {
                ui.strong("1. 3-Tier Pure-Rust ArtCraft Architecture");
                ui.label(format!("Application Version: v{APP_VERSION}"));
                ui.label(format!("Active SQLite Database Path: {}", app.db_path.display()));
                ui.monospace(
                    "L1: crates/chronocraft-core  (solver, expressions, Monte Carlo, rusqlite 0.41 bundled)\nL2: crates/chronocraft-mcp   (clap 4.5 CLI + MCP 2026-07-28 JSON-RPC 2.0 stdio server)\nL3: crates/chronocraft-app   (eframe/egui 0.37 native desktop studio + egui_kittest harness)",
                );
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("2. SQLite Persistence Schema (scenarios, chrono_meta)");
                ui.monospace(
                    "CREATE TABLE scenarios (name TEXT PRIMARY KEY, model_json TEXT NOT NULL);\nCREATE TABLE chrono_meta (id INTEGER PRIMARY KEY CHECK (id = 1), active_name TEXT NOT NULL);",
                );
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("3. Safety & Quality Containment Invariants");
                ui.label("- #![forbid(unsafe_code)] enforced across all workspace crates.");
                ui.label("- clippy::deny(unwrap_used, expect_used, panic, todo, unimplemented) enforced at workspace level.");
                ui.label("- Headless egui_kittest accessibility and rendering verification across all workbench subpages and all six help tabs.");
            });
        }
    }
}
