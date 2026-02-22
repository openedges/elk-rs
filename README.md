# elk-rs

Pure Rust port of Eclipse Layout Kernel (ELK), keeping Java-side feature/API/test parity while operating as a Rust workspace.

## Repository Layout

- `plugins/`: Rust crates mapped to ELK plugin structure (`org.eclipse.elk.*`)
- `scripts/`: quality/perf/parity automation scripts
- `perf/`: perf outputs, parity reports, baselines, policy docs
- `external/`: upstream references (`elk`, `elk-models`, `elkjs`) as submodules
- `AGENTS.md`: project progress log and next-work tracking

## Prerequisites

- Rust toolchain (stable)
- Git with submodule support
- Shell environment with standard Unix tools (`sh`, `awk`, `sed`, `rg`)

## Setup

```sh
git submodule update --init --recursive
cargo build --workspace
```

## Fast Validation

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo build --workspace --release
```

## Release Validation

Use the full release gate in:

- `RELEASE_CHECKLIST.md`

This includes:

- quality gates (`test`, `clippy`, `build`)
- Java parity/metadata parity gates
- perf regression and runtime budget gates
- go/no-go rules when baseline-only regressions appear

## Performance and Parity Docs

- Perf operation guide: `perf/README.md`
- Script catalog and env knobs: `scripts/README.md`
- Validation environment map: `VALIDATION.md`
- Baseline lifecycle policy: `perf/baselines/POLICY.md`
- Java perf triage: `perf/JAVA_PERF_TRIAGE.md`

Generated reports are written under `perf/` (for example `perf/*parity.md`, `perf/java_vs_rust.md`, `perf/recursive_runtime_budget.md`).

## External Module Policy

`external/elk` is treated as upstream reference. Local automation should run with isolated/copy mode so the original submodule tree remains unchanged after runs.

## License

License follows upstream ELK project terms. See:

- `LICENSE.md`
- `NOTICE.md`
