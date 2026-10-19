# Changelog

All notable changes to elk-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions correspond to the target ELK Java version (see `VERSIONING.md`).

## [0.11.0-ext.6] - 2026-10-19

### Fixed

- Merged internal patch `v0.11.0+rs.4` (MinWidthLayerer out-degree tie order, [#1](https://github.com/openedges/elk-rs/pull/1)) into `custom/0.11.0`. Extension features and all existing fixture outputs are unchanged.
- Verified on the merged tree: 1,045 Rust tests, extension fixtures 18/18, model parity `1,987/1,989`, phase-step trace `1,995/1,997` (unchanged from `ext.5`).
- Fixed the NAPI workflow's `aarch64-unknown-linux-gnu` cross build by linking with `aarch64-linux-gnu-gcc`; a manual run of the workflow on this branch built all six targets.
- Moved the `x86_64-apple-darwin` NAPI build to the arm64 `macos-latest` runner as a `--target` cross build (from `v0.11.0+rs.5`), removing the dependency on Intel macOS runners; the `v0.11.0+rs.5` tag run built all six targets.

### Changed

- Aligned Cargo, npm, platform packages, and optional dependencies at `0.11.0-ext.6`.

### Known Issues

- Not published to npm or crates.io; publishing is deferred and `elk-rs` is not yet on the npm registry. The linux-arm64 link failure that skipped publishing for `ext.5` (`rust-lld: --fix-cortex-a53-843419 is only supported on AArch64`) is fixed in this release. The `v0.11.0-ext.6` tag run built all six platform bindings and the WASM package; its publish step failed as intended because no `NPM_TOKEN` is configured.

## [0.11.0-ext.5] - 2026-10-14

### Fixed

- Merged the graph ownership and temporary-memory fixes from `v0.11.0+rs.3`, retaining the existing extension features and all 1,998 `ext.4` layout outputs.
- Extended allocation regression coverage to all 18 custom fixtures; verified zero retained allocation growth across these fixtures and the full model corpus.
- Aligned Cargo, npm, platform packages, and optional dependencies at `0.11.0-ext.5`.
- Routed extension publishing through npm's `ext` channel, added the missing WASM build before publishing the main package, and updated the macOS Intel CI runner.

### Compatibility and Validation

- Carried forward the low-level Rust ownership API changes documented under `0.11.0+rs.3`; JSON input/output remains unchanged from `ext.4`.
- Passed 1,044 Rust tests, 35 JS tests, 550 Java/JS comparisons, and 18 extension fixtures through each of the native and WASM backends.
- Retained existing extension/Java differences: model parity `1,987/1,989`, phase-step trace `1,995/1,997`. Verified the differences against `ext.4`; see `HISTORY.md` for details and the existing performance caveat.

## [0.11.0+custom.1]

### Added (Custom Features)

- **`ignoreEdgeInLayer` 옵션**: Network Simplex layering에서 특정 엣지를 레이어 결정에서 제외하는 옵션. `ignoreEdgeInLayer=true`인 엣지는 delta=0으로 같은 레이어에 배치되며, 배치 후 same-layer EAST→WEST 엣지를 자동 역전.
- **In-layer edge routing**: FIRST/LAST_SEPARATE 노드 간 EAST→WEST in-layer 엣지 역전 허용, `ensure_no_inacceptable_edges` 검증 비활성화, InvertedPortProcessor same-layer 더미 노드 생성 방지, External Port 노드 별도 레이어 분리.
- 커스텀 기능 fixture 테스트 18개 (`plugins/org.eclipse.elk.graph.json/tests/fixtures/`)
  - `ignore_edge_in_layer_fixtures_produce_expected_layout` (9 fixtures)
  - `in_layer_edge_routing_fixtures_produce_expected_layout` (9 fixtures)
- 커스텀 기능 단위/통합 테스트: `ignore_edge_in_layer_test`, `in_layer_edge_routing_integration_test`, `custom_cross_feature_integration_test` 등

## [0.11.0] - 2026-03-06

First stable release. Full port of ELK Java 0.11.0 with layout-identical output.

### elk-rs Internal Changes (v0.11.0+rs.3 ~ v0.11.0+rs.5)

Cargo and npm package versions remain `0.11.0`; NAPI CI builds these tags without publishing.

#### Unreleased (after v0.11.0+rs.5)

- Fixed CI: `ci.yml` now checks out submodules so tests can read `external/elk-models` and `external/elk` resources; every run had failed since 2026-03. The first run after the fix passed (1,022 tests, 0 warnings).
- Fixed the `parity.yml` workflow, which exceeded GitHub's 25-input `workflow_dispatch` limit (57 inputs) and was rejected on every push. 20 inputs remain, 37 parameters moved to `PARITY_*` env defaults, and an `env_overrides` input keeps them overridable.
- Recalibrated the performance regression thresholds in `scripts/check_perf_regression.py` to about 2x the GitHub-hosted runner measurement; the previous local-baseline values failed `radial_xlarge` on the first real `perf.yml` run, and the rerun passed all eight scenarios.
- Fixed the `parity.yml` Java compare pipeline, which had never run end to end: pin Temurin 17 and Maven 3.9.16 (the Tycho 4.0.12 build in `external/elk` does not start under the runner's Maven 3.10.0), install ripgrep for the algorithm parity checks, restore the `issue_405`..`issue_905` scenarios and the 7-column CSV in `LayeredIssueParityBenchTest` (dropped by the 6-way benchmark rework), default `java_compare_mode` to `results`, measure the Rust perf scenarios with release builds (the debug build lost the 0% Java gate on two scenarios), read ELK's generated `src-gen` option sources from the persistent Tycho worktree, and upload the parity artifacts even when a step fails. The first fully green run is 39229265896 (about 10 minutes); on the runner the Rust release build is 86-91% faster than Java on the five issue scenarios, and five of the twelve static parity reports still show pre-existing informational drift (two algorithm metadata entries missing in Rust, one option-count and two feature-pair differences).
- Refreshed `tests/baselines/*.csv` with release-build Rust measurements and JDK 17 Java measurements; the previous Rust rows were debug builds.
- Fixed `scripts/check_core_option_dependency_parity.sh`, which missed rustfmt-wrapped declarations and reported the two `topdown.sizeApproximator` dependencies as Java-only (now 10/10).
- Fixed six `scripts/check_algorithm_*_parity.sh` parsers (name, description, metadata, feature, option support, option default) that missed rustfmt-wrapped declarations: a shared pre-pass `scripts/lib/join_wrapped_rust_lines.awk` re-joins wrapped `let x =` / method-chain lines, and the Rust scan now skips `**/tests/**`. The five long-standing informational drift reports (`mrtree`/`Draw2D` missing from name/description/metadata/feature, `random` option support 4 vs 3) were parser false positives; all twelve static parity reports are `ok` on `main`. On `custom/0.11.0` the option-support and option-default reports keep one expected extension-only delta (`org.eclipse.elk.layered.layering.ignoreEdgeInLayer`).

#### v0.11.0+rs.5 (2026-10-19)

- Fixed the NAPI workflow's `aarch64-unknown-linux-gnu` job to link with `aarch64-linux-gnu-gcc` instead of the host `cc`, which failed with `rust-lld: --fix-cortex-a53-843419 is only supported on AArch64` under rustc 1.98+. A manual workflow run built all six targets.
- Moved the `x86_64-apple-darwin` NAPI build to the arm64 `macos-latest` runner as a `--target` cross build. The retired `macos-13` label had left that job queued until GitHub cancelled it 24 hours later, and GitHub ends Intel macOS runners in fall 2027; the cross-built binding is verified as x86_64 Mach-O.
- No Rust source changes since `v0.11.0+rs.4`; its quality gate results apply (1,022 tests, model parity `1,988/1,989`, phase-step trace `1,997/1,997`).

#### v0.11.0+rs.4 (2026-10-19)

- Fixed `MinWidthLayerer` to keep the input order of nodes with equal out-degree, matching Java ELK's stable `Collections.reverseOrder` sort. Previously ties were reversed, so `MIN_WIDTH` layering could differ from Java ELK and elkjs ([#1](https://github.com/openedges/elk-rs/pull/1), contributed by @simon-gd).
- Added the regression test `min_width_keeps_input_order_of_out_degree_ties` with expected layers taken from elkjs 0.11.0. No `external/elk-models` model uses `MIN_WIDTH`, so model parity and phase-step trace results are unchanged.

#### v0.11.0+rs.3 (2026-10-14)

- Fixed graph ownership cycles in ElkGraph, layered properties, crossing minimization, self-loops, splines, and network simplex; released temporary layout arenas and rectpacking rows on completion, cancellation, and panic.
- Added 16 memory regression tests; verified zero retained allocation growth across all 1,998 model inputs and repeated calls with a reused layered engine.
- Changed low-level Rust endpoint/property APIs to non-owning references: `EdgeEndpointList::iter()` now yields owned live handles, so callers must remove `.cloned()` and retain graph/node ownership. See `HISTORY.md` for the complete migration notes.
- Preserved all 1,998 pre-fix JSON results and 1,997/1,997 phase-step trace matches; retained the existing Java model-parity exception.
- Prevented npm publishing for internal `+rs.N` tags while retaining NAPI cross-platform builds.
- Recorded a 2.04% increase in mean native benchmark time and a 5.66% increase for the separately measured large Stress case; further profiling remains necessary. See `HISTORY.md` for the measurement environment and scope.

### ELK Porting

- Complete port of ELK Java 0.11.0 to Rust (~333 commits, 19 crates, ~204,500 lines)
- 13 layout algorithms: Layered (Sugiyama), Force (Fruchterman-Reingold), Stress (Stress Majorization), MrTree, Radial (Eades), Rectpacking, Disco, Spore, Vertiflex, TopdownPacking, Graphviz/Libavoid (stubs)
- Model parity: 1,988/1,989 models match Java ELK (100%), 1,998 total coverage
- Phase-step trace parity: 1,997/1,997 models match at all 50+ processor steps (100%)
- Tickets parity: 108/109 match (drift=1, same Java NaN bug)
- JS parity: 550/550 models match Java ELK via WASM/NAPI bindings

### Added

- elkjs-compatible npm package (`elk-rs@0.11.0`) with WASM + NAPI backends
- NAPI native addons for 6 platforms: `@elk-rs/darwin-arm64`, `@elk-rs/darwin-x64`, `@elk-rs/linux-x64-gnu`, `@elk-rs/linux-x64-musl`, `@elk-rs/linux-arm64-gnu`, `@elk-rs/win32-x64-msvc`
- Automatic loading priority: platform NAPI package -> local `.node` -> WASM fallback
- Browser and Node.js support via `layout()`, `knownLayoutAlgorithms()`, `knownLayoutOptions()`, `knownLayoutCategories()` API
- Web Worker support (`workerUrl` / `workerFactory` options)
- TypeScript type definitions
- 712 unit tests across 19 Rust crates + 35 Vitest tests for JS API
- 2-level parity verification: final output (Model Parity) + intermediate state (Phase-Step Trace)
- 6-way performance benchmark framework (rust_native, rust_api, java, elkjs, napi, wasm)
- Phase-step trace infrastructure for debugging divergence at each processor
- Java determinism patch for opposing self-loop routing (`ArrayListMultimap` -> `MultimapBuilder.enumKeys()`)
- CI workflows: fast checks (`ci.yml`), full parity (`parity.yml`), NAPI cross-build (`napi.yml`)

### Performance

- Rust native: **3.90x faster** than Java (26 synthetic scenario average)
- Key results (rust_native vs java):
  - `force_xlarge`: 157ms vs 947ms (**6.03x** faster)
  - `stress_xlarge`: 173ms vs 981ms (**5.67x** faster)
  - `mrtree_xlarge`: 5.82ms vs 26.6ms (**4.56x** faster)
  - `layered_xlarge`: 245ms vs 360ms (**1.47x** faster)
  - `radial_xlarge`: 11.8ms vs 17.4ms (**1.47x** faster)
- Rust wins 24/26 scenarios, Java wins 2/26 (radial_medium, radial_large)
- rust_api: 3.73x faster than Java (including JSON parse + serialize overhead)
- 28 optimization phases applied: SoA (Struct-of-Arrays) for Force/Stress/MrTree/Radial, CSR snapshot for Layered P3/P5, FxHashMap, Cow<'static, str> property keys, mimalloc allocator, EdgeRouter adjacency maps, borrow batching, lock elimination

### Fixed

- Deterministic opposing self-loop routing matching Java enum-ordinal order
- Negative graph size clamp in `ComponentGroupGraphPlacer` (`.max(0.0)` removed)
- Inside self-loop node compaction guard for nodes with children
- Greedy switch activation threshold (`>=` -> `>` matching Java)
- Port label y-cursor using `label_gap_vertical` instead of hardcoded `1.0`
- External port boundary early return in crossing minimizer
- In-layer ports integer arithmetic for crossing counting
- Stress layout `SPACING_NODE_NODE=80` default injection
- Layer constraint postprocessor node ordering (same-layer re-assignment)
- Cross-hierarchy edge graceful skip with 30s timeout (`elk_live_examples_test`)
- Various label/node size processor parity fixes

### Known Issues

- 9 models skipped (Java exceptions/timeouts/NaN output)
- `213_componentsCompaction.elkt`: Java `ComponentsCompactor` produces NaN y-coordinates (73 cases) and incorrect x-offsets (12 cases); Rust output is mathematically more accurate. See `HISTORY.md` for detailed analysis.
- `radial_medium` and `radial_large`: Java is faster due to Rc/RefCell architecture constraints in PolarCoordinateSorter per-node calls
- 20 ELKJS_DRIFT models are GWT transpilation artifacts, not elk-rs bugs
