# elk-rs

Pure Rust port of [Eclipse Layout Kernel (ELK)](https://www.eclipse.org/elk/), keeping Java-side feature/API/test parity while operating as a Rust workspace.

The working tree targets **ELK 0.12.0**; the latest published packages remain **0.11.0**
(tag `v0.11.0+rs.6`, 2026-10-19). Version 0.12.0 is being prepared for
`v0.12.0+rs.7` and has not been published. See the
[upgrade verification report](tests/upgrade_0_12_0.md) for validation status.

## npm Package

elk-rs is published as `@openedges/elk-layout`, a drop-in replacement for [elkjs](https://github.com/kieler/elkjs):

```bash
npm install @openedges/elk-layout
```

Version 0.12.0 requires Node.js 18 or later. On supported platforms, a native NAPI addon is automatically installed for best performance. Falls back to WASM on other platforms.

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
