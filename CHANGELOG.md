# Changelog

All notable changes to elk-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions correspond to the target ELK Java version (see `VERSIONING.md`).

## [0.11.0] - 2026-02-28

First stable release. Full port of ELK Java 0.11.0 with layout-identical output.

### ELK Porting

- Complete port of ELK Java 0.11.0 to Rust (220 commits, 19 crates)
- 9 layout algorithms: Layered, Stress, MrTree, Radial, Force, DisCo,
  Rect Packing, Spore Overlap, Spore Compaction
- Model parity: 1438/1438 models match Java ELK (100%)
- Phase-step trace parity: 1439/1439 models match at all 50+ processor steps
- JS parity: 550/550 models match Java ELK via WASM binding

### Added

- elkjs-compatible npm package (`elk-rs@0.11.0`) with WASM backend
- Browser and Node.js support via `layout()`, `knownLayoutAlgorithms()`,
  `knownLayoutOptions()`, `knownLayoutCategories()` API
- Web Worker support (`workerUrl` / `workerFactory` options)
- TypeScript type definitions
- 7-layer parity verification system (code quality, model parity, phase-step
  traces, API/metadata, test parity, JS parity, performance gates)
- Phase-step trace infrastructure for debugging divergence at each processor
- Java determinism patch for opposing self-loop routing (`ArrayListMultimap`
  → `MultimapBuilder.enumKeys()`)
- 653 unit tests across all crates
- CI workflows: fast checks (`ci.yml`), full parity (`parity.yml`)
- Automated parity scripts: model comparison, phase trace, API/metadata checks

### Fixed

- Deterministic opposing self-loop routing matching Java enum-ordinal order
- Negative graph size clamp in `ComponentGroupGraphPlacer` (`.max(0.0)` removed)
- Inside self-loop node compaction guard for nodes with children
- Greedy switch activation threshold (`>=` → `>` matching Java)
- Port label y-cursor using `label_gap_vertical` instead of hardcoded `1.0`
- External port boundary early return in crossing minimizer
- In-layer ports integer arithmetic for crossing counting
- Stress layout `SPACING_NODE_NODE=80` default injection
- Layer constraint postprocessor node ordering (same-layer re-assignment)
- Various label/node size processor parity fixes

### Known Issues

- 10 models skipped (9 Java exceptions/timeouts + 1 Java NaN bug)
- `213_componentsCompaction.elkt`: Java `ComponentsCompactor` produces NaN
  y-coordinates; Rust output is correct
- `elk_live_examples_test`: cross-hierarchy edge resolution not yet handled
- 20 ELKJS_DRIFT models (GWT artifacts, not elk-rs bugs)
