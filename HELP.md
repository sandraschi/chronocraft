# ChronoCraft Help & User Guide

## Get Started

**What is ChronoCraft?**
ChronoCraft is a 3-tier Pure-Rust System-Dynamics and Monte Carlo Scenario
Simulator. You draw stocks (quantities that accumulate) and flows (rates that
fill or drain them), link stocks back into flow rate expressions as feedback,
and integrate the model forward with an RK4 solver. A seeded Monte Carlo engine
replays the model up to 10,000 times with random shocks and paints p10/p50/p90
percentile bands on a fan chart.

It provides 6 integrated sub-app workbenches in the left sidebar:
1. **Canvas Studio** -- drag stocks and flows, link feedback, seed presets
   (`population`, `inventory`, `epidemic_sir`).
2. **Sliders & Parameters** -- lumped parameters with min/max sliders.
3. **Fan-Chart Viewer** -- Monte Carlo p10/p50/p90 bands on a native fan chart.
4. **Runs & Scenarios** -- save, list, and compare scenarios plus
   one-parameter sensitivity sweeps.
5. **Assistant Chat** -- local-first chat over the automation interface.
6. **Help (F1)** -- 6-tab interactive documentation.

## Functions

- **Model presets**: `population` (logistic growth with carrying capacity),
  `inventory` (stock with inflow orders and outflow demand), `epidemic_sir`
  (susceptible/infected/recovered feedback loop).
- **Rate expressions**: variable lookup (stock ids, parameter names, `t` for
  time), `+ - * /`, `min(a,b)`, `max(a,b)`, `clamp(x,lo,hi)`,
  `step(t,level)`, `pulse(t,start,width,height)`.
- **SQLite Persistence**: Auto-saves stocks, flows, links, parameters, and named
  scenarios to `%LOCALAPPDATA%\chronocraft\chronocraft.db` (or `--db <PATH>`).

## Usage

### CLI
```powershell
chronocraft-mcp status --json
chronocraft-mcp run-simulation --steps 200 --dt 0.05
chronocraft-mcp run-monte-carlo --runs 1000 --seed 42
chronocraft-mcp sensitivity --param growth --from 0.05 --to 0.2 --steps 5 --target stock_population
chronocraft-mcp save-scenario --name baseline
chronocraft-mcp compare --baseline baseline --variant challenge
```

### MCP Server (`2026-07-28` stdio)
```powershell
chronocraft-mcp --mcp
```
Exposes 11 MCP tools: `chrono_status`, `chrono_summary`, `run_simulation`,
`run_monte_carlo`, `list_scenarios`, `save_scenario`, `compare_scenarios`,
`sensitivity_sweep`, `set_param`, `add_stock`, and `add_flow`.

## Tips

- Rate expressions can read any stock id, any parameter name, and `t` (model
  time). A flow with no source stock draws from outside the model; a flow with
  no target stock drains to outside the model.
- Keep `dt` small relative to your fastest rate (try `dt 0.01` for stiff
  feedback loops) and check the analytic decay test tolerance if unsure.
- Press `Ctrl+K` for the Command Palette, `F1` for Help, and `Ctrl+MouseWheel` to zoom the UI.

## Other Crafts

ChronoCraft is part of the `sandraschi` Pure-Rust ArtCraft ecosystem alongside
`graphcraft`, `mathcraft`, `robotcraft`, `smbcraft`, `gamecraft`, and `lenscraft`.
