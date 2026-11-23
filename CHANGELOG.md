# Changelog

## [0.1.0] - 2026-11-23

### Added
- Built 3-tier Pure-Rust ArtCraft workspace (`chronocraft-core`, `chronocraft-mcp`, `chronocraft-app`).
- Added 6 hierarchical workbenches (`Canvas Studio`, `Sliders & Parameters`, `Fan-Chart Viewer`, `Runs & Scenarios`, `Assistant Chat`, `Help`).
- Added stocks-and-flows graph, flow-rate expression evaluator, RK4 solver, seeded Monte Carlo engine with p10/p50/p90 bands, scenario compare, and sensitivity sweep, with bundled SQLite persistence (`rusqlite` 0.41).
- Added 11 MCP `2026-07-28` tools and 11 CLI subcommands sharing `Command::dispatch`.
