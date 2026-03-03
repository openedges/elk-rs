# elk-rs

Pure Rust port of [Eclipse Layout Kernel (ELK)](https://www.eclipse.org/elk/), keeping Java-side feature/API/test parity while operating as a Rust workspace.

## npm Package

elk-rs is available as a drop-in replacement for [elkjs](https://github.com/kieler/elkjs):

```bash
npm install elk-rs
```

On supported platforms, a native NAPI addon is automatically installed for best performance. Falls back to WASM on other platforms.

| Platform | Package |
|---|---|
| macOS ARM64 (Apple Silicon) | `@elk-rs/darwin-arm64` |
| macOS x64 (Intel) | `@elk-rs/darwin-x64` |
| Linux x64 (glibc) | `@elk-rs/linux-x64-gnu` |
| Linux x64 (musl/Alpine) | `@elk-rs/linux-x64-musl` |
| Linux ARM64 | `@elk-rs/linux-arm64-gnu` |
| Windows x64 | `@elk-rs/win32-x64-msvc` |

```js
const ELK = require('elk-rs');
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

## Repository Layout

- `plugins/`: Rust crates mapped to ELK plugin structure (`org.eclipse.elk.*`)
  - `org.eclipse.elk.js/`: npm package — JS API, NAPI/WASM backend, TypeScript typings
  - `org.eclipse.elk.wasm/`: WASM bindings (wasm-bindgen)
  - `org.eclipse.elk.napi/`: Native Node.js addon (NAPI-RS)
- `scripts/`: quality and parity automation scripts
- `tests/`: parity reports, baselines, verification outputs, policy docs
- `external/`: upstream references (`elk`, `elk-models`, `elkjs`) as submodules

## Prerequisites

- Rust toolchain (stable, with `wasm32-unknown-unknown` target for WASM build)
- Git with submodule support
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (for WASM build)
- [@napi-rs/cli](https://napi.rs/) (for native addon build)
- Node.js 16+ (for JS package tests)

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
sh build.sh
npm install
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

Eclipse Public License 2.0 (EPL-2.0). See `LICENSE` and `NOTICE`.
