# Layout Correctness and Performance Review

Validation date: 2026-10-22. The baseline is public stable `releases/0.12.0`
at `10661c0e65186e1b2b07cfdab9ccd444e467c337`; the working branch is
`fix/layout-improvements`. The review covered downstream patches relative to
upstream commit `f736cdddacbe761217b4cac1f46f8d38bbaed39f`.
The six engine files modified here were identical between that pinned commit
and stable 0.12.0 after normalization with the same rustfmt version.
Changes that remove features for a specific consumer were excluded.
Versions, submodule pins, dependencies, and distribution settings are unchanged.

## Patch Decisions

| Area | Decision and rationale |
|---|---|
| Interactive external port locking | Included. The existing implementation reacquires a node lock while still holding it. North/south traversal also releases node, port, and edge locks before following references, allowing connections that lead back to the same object. |
| `layout_value` | Included. This additive API serves Rust consumers that already hold a JSON value and is also exported by the facade. The existing string API retains its input, error, and output behavior. |
| JSON option resolution and scalar cache | Included. Caches are scoped to one transform and reset for the next input. Only Boolean/Int/String/Double values are shared; mutable values and custom parser results are not shared. |
| Shared global defaults and early import scratch release | Included. Defaults are materialized in the final JSON with the original key order. Helpers remain `pub(crate)` so internal optimizations do not become public API. |
| Network-simplex optimizations | Included. Incremental cut calculation is used only within the exact integer range and with a complete, connected spanning tree. Other cases retain full calculation in the original order. |
| Bit vectors for node and port order relations | Included. Relation updates match the previous set implementation in regression tests. Full Java phase comparison and targeted performance measurements also passed. |
| WASM mutex backend replacement | Excluded. The direct cause of the confirmed deadlock is fixed; no separate evidence established a need to replace the mutex backend. |
| Restricting providers and crates to layered layout | Excluded. This conflicts with upstream support for the full set of algorithms. |
| Removing JSON5 or disabling `lenient` | Excluded. This would break the upstream input contract. Existing JSON5 tests remain in place and pass. |
| Manifest pins, dependency removal, and deletion of unused functions or performance tools | Excluded. These are consumer-specific vendoring or distribution choices, or cleanup unrelated to the selected correctness and performance changes. |
| Reference-cycle cleanup patches | Excluded. The upstream fixes available since rs.3 are retained without applying duplicate patches. |

## Validation

- `cargo build --workspace`: zero errors and warnings.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: **1,050 passed / 0 failed / 2 ignored**.
  This adds 18 regression tests to the stable baseline of 1,032.
- JSON library tests with `preserve_order`: **14/14 passed**.
- `cargo check -p org-eclipse-elk-wasm --target wasm32-unknown-unknown`: passed.
- Model parity: **1,998 successful Rust executions**; of 1,989 comparable Java
  results, **1,988 matched and 1 had an existing difference**. There were
  **9 skipped comparisons and zero execution errors or timeouts**.
  The sole model difference is `213_componentsCompaction.elkt`: 73 Java
  non-finite coordinates and 12 finite x-coordinate differences caused by the
  simplified Rust component-compaction implementation. The candidate Rust
  output is identical to the stable baseline for this model.
- Phase trace: the existing comparator reported **1,997/1,997 matched**.
  A subsequent audit found that it misses NaN-versus-finite differences.
  An in-memory finite-aware check of `213_componentsCompaction.elkt` changes
  its result from 229 matching steps to 207 matching and 22 differing steps,
  starting at step 207. The original result is not proof of full equivalence.
  The subsequent [component compaction follow-up](component_compaction.md) fixes
  the comparator, implements the algorithm, and replaces this historical phase
  result with finite-aware counts.
  `588_hierarchicalLayoutNPE.elkt` has no Java trace because Java throws an NPE.
  This single Rust-only case is not counted as a successful comparison.
- Java exports were reused from validation artifacts with the same pins:
  Java 0.12.0 (`32331291`) and elk-models `ac5af2ba`.
  Rust layouts and traces were generated again from the modified code.
- rustfmt checks for the changed Rust files and `git diff --check` passed.
  Pre-existing formatting differences elsewhere in the repository were not
  included in this change.

Regression coverage includes cut values after each integer pivot, bitwise
equality for fractional cuts, fallback for incomplete postorder traversal,
subtree order, relation sets crossing 64-bit boundaries, port coordinates on
all four sides, and lock re-entry. It also covers defaults, aliases, errors,
cleanup on success/error/panic, and existing results from the 10 providers
registered by the test.
The defaults reference preserves the original merge implementation in test
code instead of reusing the optimized merge. The existing DisCo default
polyomino reflection error was confirmed to be identical through both paths;
that separate defect remains unresolved.

## Performance Measurements

Both variants were measured on the same MacBook Pro `Mac15,6`, with an
Apple M3 Pro (11 cores: 5P+6E), 36 GiB RAM, macOS 27.0.1 (26A434), and
Rust 1.97.1. Both used the system allocator and
`CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0`.
The same benchmark/probe source was linked separately against the baseline
and candidate libraries. The resulting binaries were retained and measured
in three rounds with alternating execution order. Each scenario used
3 warmup iterations and 10 measured iterations. Times below are medians of
the per-round means. No other builds or tests ran during measurement.

| Input | Baseline ms | Candidate ms | Change | Peak live heap delta in MiB: baseline → candidate |
|---|---:|---:|---:|---:|
| global_defaults_1024 | 57.343 | 42.831 | -25.3% | 26.28 → 21.58 |
| global_defaults_4096 | 245.372 | 188.446 | -23.2% | 105.18 → 86.34 |
| ordered_simplex_128 | 10.855 | 8.256 | -23.9% | 5.25 → 4.71 |
| ordered_simplex_512 | 137.557 | 52.623 | -61.7% | 20.89 → 18.71 |

`global_defaults` uses two ports and one label per node, with 11 global
options. The `fixed` algorithm isolates the effect of JSON processing.
`ordered_simplex` uses a branching tree with the same node contents and
applies `NODES_AND_EDGES` and `NETWORK_SIMPLEX` together.
These measurements cover the combined patch set; they do not isolate each
optimization's contribution or establish a customer workload SLA.
The counting allocator measures the peak increase in live allocated bytes
above the warmed baseline. This is neither RSS nor a memory limit.
All four inputs had zero additional retained allocation after the iterations.

The standard synthetic benchmark results are listed below. Improvements on
the targeted large inputs should not be generalized to every input. Both
relative changes and absolute times are retained for the smaller scenarios.

- `rust_native`: the arithmetic mean of the 26 scenario medians changed from
  32.234 to 31.637 ms (-1.85%). The largest relative increase was
  `crossmin_layer_sweep`: 2.624 to 2.641 ms (+0.68%).
- `rust_api`: the arithmetic mean of the 26 scenario medians changed from
  33.779 to 33.648 ms (-0.39%). The largest relative increase was
  `stress_medium`: 5.363 to 5.545 ms (+3.38%).

Raw rounds: [measurements.csv](layout_improvements/measurements.csv).
Per-scenario medians: [summary.json](layout_improvements/summary.json).
Supplemental probe source: [patch_probe.rs](layout_improvements/patch_probe.rs).

## Reproduction

1. Build the baseline commit and candidate code separately with Rust 1.97.1:
   `CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0 cargo build -p org-eclipse-elk-graph-json --bin perf_benchmark`.
2. Save each build's `perf_benchmark` under a distinct name. Run each binary
   with `--engine rust_native` and `--engine rust_api`, using
   `--mode synthetic --iterations 10 --warmup 3 --output <csv>`.
3. Link the same supplemental probe source against each build's graph-JSON
   `.rlib`:
   `rustc --edition=2021 -C opt-level=3 -L dependency=target/debug/deps --extern org_eclipse_elk_graph_json=<rlib-from-that-build> tests/layout_improvements/patch_probe.rs -o <probe>`.
   Save each binary's CSV output. Do not mix baseline and candidate libraries.
4. Measure three rounds in baseline/candidate, candidate/baseline, and
   baseline/candidate order, then compare the per-scenario medians.
5. For functional validation, follow the model and phase procedures in
   `TESTING.md` §2.B. Raw artifacts from this run are in
   `/tmp/elk-upstream-review/`. The validation summary and raw performance
   measurements are retained in this document and its adjacent files so
   they remain available independently of that temporary directory.

`layout_value` takes ownership of the input value. Unlike the string path, it
avoids serialization and reparsing. Depending on floating-point round-trip
settings, this can produce a difference of about 1 ULP. Consumers that require
the existing text boundary can continue using `layout_json`.
