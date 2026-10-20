# Changelog

All notable changes to elk-rs are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions correspond to the target ELK Java version (see `VERSIONING.md`).

## [0.12.0] - 2026-10-20

ELK Java 0.12.0 port. Release tag: `v0.12.0+rs.7`.

### ELK Porting

- Ported layered node-flexibility repetition (`NodePlacementRepeater`) and corrected alternating layer unzipping for the upstream 0.12.0 behavior; added nine feature fixtures.
- Pinned Java ELK `v0.12.0` (`323312916048544e4e594eab3303cdb0d01e2929`) and elkjs `0.12.0` (`ff5771d7165445c42c408bb8a090c8035272218c`); aligned Cargo/npm versions and pinned the test dependency to exact elkjs `0.12.0`.
- Verified 1,032 Rust tests passed, zero failed, two ignored, with warning-free workspace build and Clippy. Fresh 0.12.0 NAPI and browser WASM each matched Java on 550/550 JSON models (530 `PASS`, 20 `ELKJS_DRIFT`). Full model and phase-step trace verification status is recorded in [the upgrade report](tests/upgrade_0_12_0.md).

### Fixed

- Initialized browser direct WASM lazily through the existing Promise API; verified concurrent first requests and browser worker layouts in Chrome.
- Compared Java baselines for every JS parity model, even when the two JS engines agree; matched exact manifest paths and made `--require-java` fail on missing baselines.
- Added a Node.js CommonJS WASM bundle in `dist/wasm-node/` for the Node.js 18+ fallback, alongside browser ESM WASM in `dist/wasm/`; included both bundles in npm packaging and release checks.

### Changed

- Raised the npm package minimum from Node.js 16 to Node.js 18: the generated WASM requires reference types, unavailable on Node.js 16 without experimental flags. Verified the isolated CommonJS fallback on Node.js 18.0.0 and 20.20.2; development remains on pinned Node.js 24.
- Matched upstream's `EPL-2.0 OR GPL-3.0-or-later` metadata while preserving the conditional EPL secondary-license notice and attribution; included applicable license notices in Cargo and npm packages.

## [0.11.0] - 2026-03-06

First stable release. Full port of ELK Java 0.11.0 with layout-identical output.

### elk-rs Internal Changes (v0.11.0+rs.3 ~ v0.11.0+rs.6)

Cargo and npm package versions remained `0.11.0`. Tags through `+rs.5` were
build-only; `v0.11.0+rs.6` first published the packages on 2026-10-19.

#### v0.11.0+rs.6 and subsequent release infrastructure maintenance

- Fixed CI: `ci.yml` now checks out submodules so tests can read `external/elk-models` and `external/elk` resources; every run had failed since 2026-03. The first run after the fix passed (1,022 tests, 0 warnings).
- Fixed the `parity.yml` workflow, which exceeded GitHub's 25-input `workflow_dispatch` limit (57 inputs) and was rejected on every push. 20 inputs remain, 37 parameters moved to `PARITY_*` env defaults, and an `env_overrides` input keeps them overridable.
- Recalibrated the performance regression thresholds in `scripts/check_perf_regression.py` to about 2x the GitHub-hosted runner measurement; the previous local-baseline values failed `radial_xlarge` on the first real `perf.yml` run, and the rerun passed all eight scenarios.
- Fixed the `parity.yml` Java compare pipeline, which had never run end to end: pin Temurin 17 and Maven 3.9.16 (the Tycho 4.0.12 build in `external/elk` does not start under the runner's Maven 3.10.0), install ripgrep for the algorithm parity checks, restore the `issue_405`..`issue_905` scenarios and the 7-column CSV in `LayeredIssueParityBenchTest` (dropped by the 6-way benchmark rework), default `java_compare_mode` to `results`, measure the Rust perf scenarios with release builds (the debug build lost the 0% Java gate on two scenarios), read ELK's generated `src-gen` option sources from the persistent Tycho worktree, and upload the parity artifacts even when a step fails. The first fully green run is 39229265896 (about 10 minutes); on the runner the Rust release build is 86-91% faster than Java on the five issue scenarios, and five of the twelve static parity reports still show pre-existing informational drift (two algorithm metadata entries missing in Rust, one option-count and two feature-pair differences).
- Refreshed `tests/baselines/*.csv` with release-build Rust measurements and JDK 17 Java measurements; the previous Rust rows were debug builds.
- Fixed `scripts/check_core_option_dependency_parity.sh`, which missed rustfmt-wrapped declarations and reported the two `topdown.sizeApproximator` dependencies as Java-only (now 10/10).
- Fixed six `scripts/check_algorithm_*_parity.sh` parsers (name, description, metadata, feature, option support, option default) that missed rustfmt-wrapped declarations: a shared pre-pass `scripts/lib/join_wrapped_rust_lines.awk` re-joins wrapped `let x =` / method-chain lines, and the Rust scan now skips `**/tests/**`. The five long-standing informational drift reports (`mrtree`/`Draw2D` missing from name/description/metadata/feature, `random` option support 4 vs 3) were parser false positives; all twelve static parity reports are `ok` on `main` (`parity.yml` run 39242090738). On `custom/0.11.0` the option-support and option-default reports keep one expected extension-only delta (`org.eclipse.elk.layered.layering.ignoreEdgeInLayer`).
- The Java parity scripts now pass `-Ddash.skip=true` to every Maven invocation of the ELK Tycho build (`ejc_mvn_dash_args` in `scripts/java/elk_java_common.sh`; `JAVA_PARITY_DASH_LICENSE_CHECK=true` re-enables it). ELK's Eclipse Dash license-check queries the Eclipse Foundation API and aborted a `parity.yml` run with HTTP 504; the parity pipeline does not use its output.
- Changed the release/version policy so that `+rs.N` tags are the published releases and `-ext.N` extension tags are never published (`VERSIONING.md` § 1, § 3, § 6). Registries cannot carry `+rs.N` (npm strips build metadata, crates.io treats `0.11.0+rs.5` and `0.11.0+rs.6` as the same version), so published fixes bump the patch version (`0.11.0` -> `0.11.1`, the way elkjs does); a `+rs.N` tag that does not bump the version is build-only because both publish scripts skip versions already on the registry.
- Prepared the 17 library crates for crates.io: `description`, `repository`, `keywords` and `categories` in every manifest, intra-workspace dependencies moved to `[workspace.dependencies]` with exact `=0.11.0` pins (dev-dependencies stay path-only so the test-only cycles publish), `publish = false` on the `napi`/`wasm` cdylib bindings, and `"private": true` on the `elk-rs-napi` helper manifest. `cargo publish --workspace --dry-run` packages all 17 crates (largest `org-eclipse-elk-graph-json`, 1.1 MiB).
- Added `scripts/bump_version.py` (sets the version in `Cargo.toml`, `Cargo.lock`, the npm manifests, lockfiles and the six platform packages; `--check` verifies they agree), `scripts/publish_crates.py` (publishes the crates missing on crates.io in one dependency-ordered `cargo publish`, retrying after the new-crate rate limit) and `scripts/publish_npm.sh` (platform packages then `elk-rs`, skipping versions already on npm and refusing to publish a platform package without its binary).
- Changed `napi.yml` to publish on `v*` tags that are not `-ext.` tags (previously `+rs.` tags were excluded) through `scripts/publish_npm.sh` with dist-tag `latest`, and to build the WASM bundle in the publish job: `files` lists `dist/wasm/`, but the `main` workflow never built it, so the package would have shipped without its WASM fallback. Added `.github/workflows/cargo-publish.yml` (same trigger, `scripts/publish_crates.py`; `workflow_dispatch` runs a dry run). Both workflows check that every manifest carries the tag's base version.
- Fixed the stale root version (`0.1.0`) in `plugins/org.eclipse.elk.js/package-lock.json`.
- Renamed the npm packages to `@openedges/elk-layout` and `@openedges/elk-layout-<platform>` (binary `elk-layout.<triple>.node`): `elk-rs` is taken on crates.io by an unrelated crate, so the Rust facade is `elk-layout` and the npm packages use the same name under the `@openedges` scope. The loader order (platform package -> local `.node` -> WASM) and the elkjs-compatible API are unchanged; only the package name in `require()`/`import` changes.
- Added the `elk-layout` crate (`crates/elk-layout`): re-exports the 17 `org-eclipse-elk-*` crates as `elk_layout::graph`, `core`, `graph_json`, `conn_gmf` and `alg::*`, and exposes `layout_json`/`known_layout_*` at the top level; `cargo add elk-layout` is the one-line Rust dependency. 18 crates are published to crates.io.
- Published elk-rs `0.11.0` to crates.io (the `elk-layout` facade and the 17 `org-eclipse-elk-*` crates) and npm (`@openedges/elk-layout` and the six platform packages) on 2026-10-19 from tag `v0.11.0+rs.6`, with the token-based workflows (five crates from a local `cargo login`, the other 13 through the tag run's rate-limit retry loop).
- Replaced `napi.yml` and `cargo-publish.yml` with one `release.yml` (the decl-lang layout): a `preflight` job checks that every manifest carries the tag's version and that `mise.toml` and `rust-toolchain.toml` agree; every `v*` tag builds the six NAPI binaries and the WASM bundle; release tags (no `-ext.`) publish the missing npm packages and crates through Trusted Publishing (GitHub Actions OIDC, environments `npm` and `crates-io`, `rust-lang/crates-io-auth-action`) with the old secrets as the transition fallback, and, once both publishes succeeded, attach the `.node` binaries and a WASM tarball to a GitHub Release. A manual run (from a branch or a tag) builds everything and dry-runs both publish scripts; `-ext.` tags are build-only.
- Pinned the toolchains in `rust-toolchain.toml` and `mise.toml` (Rust 1.97.1, Node 24); `ci.yml`, `parity.yml` and `perf.yml` install that Rust channel instead of `stable`.

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
- Model parity: 1,988/1,989 models match Java ELK (one accepted Java bug), 1,998 total coverage
- Phase-step trace parity: 1,997/1,997 models match at all 50+ processor steps (100%)
- Tickets parity: 108/109 match (drift=1, same Java NaN bug)
- JS parity: 550/550 models match Java ELK via WASM/NAPI bindings

### Added

- elkjs-compatible npm package (`@openedges/elk-layout@0.11.0`, named `elk-rs` until 2026-10-19) with WASM + NAPI backends
- NAPI native addons for 6 platforms: `@openedges/elk-layout-darwin-arm64`, `@openedges/elk-layout-darwin-x64`, `@openedges/elk-layout-linux-x64-gnu`, `@openedges/elk-layout-linux-x64-musl`, `@openedges/elk-layout-linux-arm64-gnu`, `@openedges/elk-layout-win32-x64-msvc`
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
