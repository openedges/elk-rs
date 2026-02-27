# elk-rs

Pure Rust port of Eclipse Layout Kernel (ELK), keeping Java-side feature/API/test parity while operating as a Rust workspace.

## npm Package

elk-rs is available as a drop-in replacement for [elkjs](https://github.com/nickkraft/elkjs):

```bash
npm install elk-rs
```

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
  - `org.eclipse.elk.js/`: npm package — JS API, WASM backend, TypeScript typings
  - `org.eclipse.elk.wasm/`: WASM bindings (wasm-bindgen)
  - `org.eclipse.elk.napi/`: Native Node.js addon (NAPI-RS)
- `scripts/`: quality and parity automation scripts
- `parity/`: parity reports, baselines, parity outputs, policy docs
- `external/`: upstream references (`elk`, `elk-models`, `elkjs`) as submodules

## Prerequisites

- Rust toolchain (stable, with `wasm32-unknown-unknown` target for WASM build)
- Git with submodule support
- Shell environment with standard Unix tools (`sh`, `awk`, `sed`, `rg`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) (for WASM build)
- Node.js 16+ (for JS package tests)

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

## Validation Flow

All validation follows the gate execution order defined in `parity/PARITY.md`.

### 1. Static Checks (Code Quality)

Basic build, lint, and test health. Run after every code change.

```sh
cargo build --workspace            # zero errors/warnings
cargo clippy --workspace --all-targets  # zero warnings
cargo test --workspace             # zero failures
```

### 2. Full Model Parity

Compares complete layout output of 1448 models between Java ELK and elk-rs.
This is the primary functional gate.

```sh
# Full run (Java export + Rust layout + comparison):
sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity

# Reuse existing Java baseline (faster):
MODEL_PARITY_SKIP_JAVA_EXPORT=true \
  sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity
```

Output: `parity/model_parity/report.md`, `parity/model_parity/diff_details.tsv`

### 3. Phase-Step Trace Verification

Compares intermediate state after each of 50+ layered pipeline processors.
Pinpoints at which processing step divergence first occurs.

```sh
# Generate Java traces:
sh scripts/run_java_phase_trace.sh <input_dir> <output_dir>

# Generate Rust traces:
cargo run --release --bin model_parity_layout_runner \
  -- --trace-dir <output_dir> <input.json>

# Compare and summarize:
python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch
python3 scripts/summarize_phase_gate.py \
  --java-manifest ... --rust-manifest ... \
  --java-trace-dir ... --rust-trace-dir ... \
  --compare-json ... --output-md parity/model_parity/phase_gate_latest.md
```

Output: `parity/model_parity/phase_gate_latest.md`

### Release Validation

Use the full release gate in `RELEASE_CHECKLIST.md`, which includes:

- all static checks above
- Java parity/metadata parity gates
- parity regression and runtime budget gates
- go/no-go rules when baseline-only regressions appear

## Parity Docs

- Parity operation guide: `parity/README.md`
- Script catalog and env knobs: `scripts/README.md`
- Parity verification system: `parity/PARITY.md`
- Baseline lifecycle policy: `parity/baselines/POLICY.md`
- Java parity triage: `parity/java_parity_triage.md`
- Version management and porting policy: `VERSIONING.md`

Generated reports are written under `parity/` (for example `parity/*_parity.md`, `parity/model_parity/report.md`).

## External Module Policy

`external/elk`, `external/elk-models`, `external/elkjs` are upstream reference submodules.

- **Do NOT modify** any files under `external/`. These directories must remain identical to their upstream commits.
- Local automation should run with isolated/copy mode so the original submodule tree remains unchanged after runs.
- Java parity exports use `JAVA_PARITY_EXTERNAL_ISOLATE=true` (default) to build in a temporary worktree, leaving `external/elk` untouched.

## License

License follows upstream ELK project terms. See:

- `LICENSE.md`
- `NOTICE.md`
