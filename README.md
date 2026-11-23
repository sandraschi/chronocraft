# ChronoCraft

**Pure-Rust System-Dynamics and Monte Carlo Scenario Simulator: stocks-and-flows models with feedback loops, RK4 integration, 10k-run percentile bands, and baseline-versus-variant scenario compare** (`L1 chronocraft-core` + `L2 chronocraft-mcp` + `L3 chronocraft-app`).

## Overview

ChronoCraft models quantities that accumulate and drain over time (stocks) through
rates (flows) with feedback links and lumped parameters. It bundles 6 hierarchical
workbenches in one unified workspace:

1. **Canvas Studio** -- drag stocks and flows, link feedback from stocks into flow
   rate expressions, and seed built-in presets (`population`, `inventory`, `epidemic_sir`).
2. **Sliders & Parameters** -- lumped parameters with min/max sliders bound to live
   simulation state.
3. **Fan-Chart Viewer** -- Monte Carlo p10/p50/p90 percentile bands painted on a
   native fan chart.
4. **Runs & Scenario Manager** -- save, list, and compare scenarios (baseline vs
   variant end-state diff) plus one-parameter sensitivity sweeps.
5. **Assistant Chat** -- local-first chat over the automation interface.
6. **Help (F1)** -- 6-tab interactive documentation.

## Feature Matrix

| Capability | Status | Crate / Module |
|---|---|---|
| Stocks-and-flows graph (stocks, flows, feedback links, parameters, seeded shocks) | `Implemented` | `chronocraft-core::model` |
| Flow-rate expression evaluator (variables, `+ - * /`, `min`/`max`/`clamp`, `step`/`pulse`) | `Implemented` | `chronocraft-core::expr` |
| RK4 ODE solver over the stocks-and-flows graph | `Implemented` | `chronocraft-core::sim` |
| Monte Carlo engine (up to 10k runs, p10/p50/p90 bands, seeded RNG) | `Implemented` | `chronocraft-core::monte` |
| Scenario compare (baseline vs variant end-state diff) + sensitivity sweep | `Implemented` | `chronocraft-core` |
| Bundled SQLite persistence (`%LOCALAPPDATA%\chronocraft\chronocraft.db`) | `Implemented` | `chronocraft-core::db` |
| CLI (`--json`) + MCP `2026-07-28` Stdio Server (`11` tools) | `Implemented` | `chronocraft-mcp` |
| Native `eframe`/`egui` 0.37 Studio (6 Workbenches, WCAG AAA Dark/Light) | `Implemented` | `chronocraft-app` |

## Quick Start

```powershell
# Launch desktop studio
.\start.ps1

# Run headless smoke test
.\start.ps1 -Smoke

# Run CLI commands
cargo run -p chronocraft-mcp -- status --json
cargo run -p chronocraft-mcp -- run-simulation --steps 200
cargo run -p chronocraft-mcp -- run-monte-carlo --runs 1000 --seed 42
cargo run -p chronocraft-mcp -- sensitivity --param growth --from 0.05 --to 0.2 --steps 5

# Run MCP 2026-07-28 stdio server
cargo run -p chronocraft-mcp -- --mcp
```

## License

Dual-licensed under `MIT OR Apache-2.0`.
