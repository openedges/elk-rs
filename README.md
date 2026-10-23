# elk-rs

Pure Rust port of [Eclipse Layout Kernel (ELK)](https://www.eclipse.org/elk/), keeping Java-side feature/API/test parity while operating as a Rust workspace.

This branch is **custom/0.12.0**, with extension package version
**0.12.1-ext.1** and the fixes from stable **0.12.1** on Java ELK **0.12.0**.
See [custom features](CUSTOM_FEATURES.md), [version validation](tests/release_0_12_1_ext_1.md),
and [versioning policy](VERSIONING.md). Extension builds use a source checkout;
`npm install @openedges/elk-layout` installs the published stable package.

The new `v0.12.1-ext.1` tag applies the stable-base naming policy. Its runtime
is unchanged from the preserved `v0.12.0-ext.2`; package versions and release
documentation identify the new version. Java compatibility remains `0.12.0`.
See the [policy transition](VERSIONING.md#1-version-management).

## npm Package

elk-rs is published as `@openedges/elk-layout`, a drop-in replacement for [elkjs](https://github.com/kieler/elkjs):

```bash
npm install @openedges/elk-layout
```

Version 0.12.x requires Node.js 18 or later. On supported platforms, a native NAPI addon is automatically installed for best performance. Falls back to WASM on other platforms.

| Platform | Package |
|---|---|
| macOS ARM64 (Apple Silicon) | `@openedges/elk-layout-darwin-arm64` |
| macOS x64 (Intel) | `@openedges/elk-layout-darwin-x64` |
| Linux x64 (glibc) | `@openedges/elk-layout-linux-x64-gnu` |
| Linux x64 (musl/Alpine) | `@openedges/elk-layout-linux-x64-musl` |
| Linux ARM64 | `@openedges/elk-layout-linux-arm64-gnu` |
| Windows x64 | `@openedges/elk-layout-win32-x64-msvc` |

```js
const ELK = require('@openedges/elk-layout');
const elk = new ELK();

elk.layout({
  id: 'root',
  layoutOptions: { 'elk.algorithm': 'layered' },
  children: [
    { id: 'n1', width: 30, height: 30 },
    { id: 'n2', width: 30, height: 30 },
  ],
  edges: [{ id: 'e1', sources: ['n1'], targets: ['n2'] }]
}).then(console.log);
```

See [`plugins/org.eclipse.elk.js/README.md`](plugins/org.eclipse.elk.js/README.md) for full API documentation.

## Rust Crates

The `elk-layout` crate re-exports the whole port and the elkjs-compatible JSON API:

```bash
cargo add elk-layout
```

```rust
let laid_out = elk_layout::layout_json(graph_json, "{}")?;   // same JSON as elkjs
```

For an already constructed `serde_json::Value`, use
`elk_layout::layout_value(graph_value, options_json) -> Result<Value, String>`.
It consumes the graph and returns the laid-out value without a graph text
round trip. The string API, JSON5 import builder, and algorithm registrations
remain available. Connected-component compaction and its Java reference checks are
documented in [the compaction follow-up](tests/component_compaction.md).
The [full phase-trace report](tests/phase_trace_audit.md) records the corrected
Java reference, raw-state comparison contract, and all 1,998 input outcomes.
See [the patch review and validation](tests/layout_improvements.md)
for compatibility and performance evidence.

Underneath, the ELK plugins are published one to one as `org-eclipse-elk-*` crates (`elk_layout::core`, `elk_layout::graph`, `elk_layout::alg::layered`, ...), all at the same version as the npm package and pinned to each other exactly. See [`crates/elk-layout/README.md`](crates/elk-layout/README.md); release and publishing rules are in [`VERSIONING.md`](VERSIONING.md).

## Repository Layout

- `plugins/`: Rust crates mapped to ELK plugin structure (`org.eclipse.elk.*`)
  - `org.eclipse.elk.js/`: npm package — JS API, NAPI/WASM backend, TypeScript typings
  - `org.eclipse.elk.wasm/`: WASM bindings (wasm-bindgen)
  - `org.eclipse.elk.napi/`: Native Node.js addon (NAPI-RS)
- `scripts/`: quality and parity automation scripts
- `tests/`: parity reports, baselines, verification outputs, policy docs
- `external/`: upstream references (`elk`, `elk-models`, `elkjs`) as submodules

## Prerequisites

- Rust 1.97.1 (`rust-toolchain.toml` and `mise.toml`), with `wasm32-unknown-unknown` for WASM builds
- Git with submodule support
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (for WASM build)
- [@napi-rs/cli](https://napi.rs/) (for native addon build)
- Node.js 24 (`mise.toml`) for development and tests; the 0.12.0 JS package requires Node.js 18+

For full validation (model parity, phase traces), additional tools are required.
See `TESTING.md` § 1 for the complete environment setup.

## Setup

```sh
git submodule update --init --recursive
cargo build --workspace
```

### Building the JS/WASM Package

```sh
cd plugins/org.eclipse.elk.js
sh build.sh                    # NAPI + browser ESM WASM + Node.js CommonJS WASM
npm install --ignore-scripts
npm test
```

## Validation

Run after every code change:

```sh
cargo build --workspace            # zero errors/warnings
cargo clippy --workspace --all-targets  # zero warnings
cargo test --workspace             # zero failures
```

For model parity, phase-step traces, API/metadata checks, performance gates,
and release procedures, see `TESTING.md`.

## Documentation

| Document | Description |
|----------|-------------|
| `CHANGELOG.md` | Release history and notable changes |
| `TESTING.md` | Testing and validation guide (setup, verification items, release checklist) |
| `VERSIONING.md` | Version management, porting policy, and changelog rules |
| `tests/PARITY.md` | Parity verification system (architecture, env vars, exceptions) |
| `tests/README.md` | Parity operation guide and script reference |
| `scripts/README.md` | Script catalog and environment knobs |
| `tests/baselines/POLICY.md` | Baseline lifecycle policy |
| `tests/java_parity_triage.md` | Java parity failure triage |

Generated reports are written under `tests/` (e.g., `tests/*_parity.md`, `tests/model_parity/report.md`).

## External Module Policy

`external/elk`, `external/elk-models`, `external/elkjs` are upstream reference submodules.

- **Do NOT modify** any files under `external/`. These directories must remain identical to their upstream commits.
- Local automation should run with isolated/copy mode so the original submodule tree remains unchanged after runs.
- Java parity exports use `JAVA_PARITY_EXTERNAL_ISOLATE=true` (default) to build in a temporary worktree, leaving `external/elk` untouched.

## License

Eclipse Public License 2.0. This Source Code may also be made available under
GPL-3.0-or-later when the conditions for secondary licensing in EPL-2.0 are
satisfied, matching Java ELK 0.12.0 (`EPL-2.0 OR GPL-3.0-or-later`).
See [LICENSE](LICENSE) and [NOTICE](NOTICE).
