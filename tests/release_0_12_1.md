# elk-rs 0.12.1 Release Validation

Date: 2026-10-23. Package version: **0.12.1**. Java/elkjs reference: **0.12.0**.
The runtime consists of `1d9f3afd` and `4b9a1f56` on top of stable `10661c0e`.
The package patch changes no upstream pins or third-party dependency versions.
Changes merge through a PR into `main`, then advance the existing
`releases/0.12.0` line and receive annotated tag `v0.12.1`.
This new package version is publishable on npm/crates.io; a `+rs.N` tag on
already-published 0.12.0 would not deliver a new registry package.

## Correctness

- Workspace build and Clippy with `-D warnings`: passed without warnings.
- Workspace tests: **1,081 passed, zero failed, two ignored** after the version bump.
- Strict full-corpus gate: **1,998/1,998 passed**, comprising **1,997 matching
  successful layouts and one matching-exception test**. All **196,407 raw
  intermediate states** matched. No missing input or non-finite successful result.
- Comparator/accounting tests: **21 passed**. The full-gate runner smoke check
  passed two successful cases and one matching exception, with 135 states.
- The full corpus was run on the same runtime before the version-only bump;
  detailed provenance and reference patches are in [the phase audit](phase_trace_audit.md).
  Java 0.12.0 uses three explicit reference fixes: self-loop determinism,
  zero-size resize, and zero-height scanline ordering. These results do not
  assert equivalence to unmodified upstream Java.
- Existing repository-wide rustfmt drift was not reformatted. Newly added Rust
  modules and changed regions were checked during implementation. Whitespace
  in Java `.patch` context preserves the original upstream lines.

## Performance

MacBook Pro `Mac15,6`, Apple M3 Pro (11 cores: 5P+6E), 36 GiB RAM,
macOS 27.0.1 (26A434), Rust 1.97.1, system allocator.
All three versions use `CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0`.
Retained binaries from the stable and first-patch validations use the same
benchmark/probe source. Binary hashes are in the summary JSON.

Three rounds alternate stable/previous/current and reverse execution order.
Each standard scenario uses 3 warmups and 10 measured iterations. A scenario's
reported time is the median of its three per-round means; the aggregate below
is the arithmetic mean across 26 scenarios, not total application latency.
No builds or tests ran concurrently with these measurements.

| API | Stable `10661c0e` ms | First patch `1d9f3afd` ms | Final `4b9a1f56` ms | Final vs stable |
|---|---:|---:|---:|---:|
| Native graph API | 32.0041 | 31.6736 | 31.5127 | -1.54% |
| JSON API | 33.4439 | 33.2829 | 33.4509 | +0.02% |

All 52 standard comparisons against stable remain below +5%. The largest
increase is JSON `layered_large`, +3.92%; native's largest is +0.32%.
Against the first patch, JSON `force_large` increased +5.36% in this run,
while remaining +1.35% against stable. These short timings include noise;
they do not prove that every input has become faster.

| Targeted workload | Stable ms | Final ms | Change | Peak live heap bytes: stable → final |
|---|---:|---:|---:|---:|
| global_defaults_1024 | 56.188 | 41.953 | -25.34% | 27561232 → 22625351 |
| global_defaults_4096 | 242.836 | 185.655 | -23.55% | 110293264 → 90534984 |
| ordered_simplex_128 | 10.801 | 8.290 | -23.25% | 5509300 → 4938152 |
| ordered_simplex_512 | 136.859 | 51.956 | -62.04% | 21908715 → 19618399 |

All targeted workloads retained zero additional allocation after the measured
iterations. Peak live heap delta is allocator accounting, not RSS.
These are synthetic inputs, not a customer workload SLA or a new Java speedup claim.
Raw CSV rounds and [the summary JSON](release_0_12_1/performance-summary.json)
are preserved under `tests/release_0_12_1/`.

Reproduce the standard measurements using the same binary at each revision:

```sh
CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0 cargo build -p org-eclipse-elk-graph-json --bin perf_benchmark
# Run each engine for each retained revision in alternating order, three rounds.
<retained-perf-binary> --engine rust_native --mode synthetic --iterations 10 --warmup 3 --output <round.csv>
<retained-perf-binary> --engine rust_api --mode synthetic --iterations 10 --warmup 3 --output <round.csv>
```

The targeted probe source and its fixture definitions are documented in
[the initial patch review](layout_improvements.md).

## Package and release gates

- All **12 strict metadata/phase-wiring reports** passed. Generated Java metadata
  came from the clean pinned 0.12.0 build; stale ignored 0.11.0 `src-gen` files in
  the local checkout were not used. No submodule source was changed.
- Full release-profile workspace build, NAPI arm64 build, browser WASM build,
  and Node.js WASM build passed.
- Fresh native and browser WASM each matched **550/550** Java JSON results:
  530 `PASS`, 20 `ELKJS_DRIFT`, zero missing Java baselines and zero failures.
- JS tests: **49/49 passed**. Two old elkjs#8 expectations had accepted an edge
  silently dropped by Rust. Restored upstream's `UnsupportedGraphException`
  assertions for `SEPARATE_CHILDREN`; both `INCLUDE_CHILDREN` layout tests still
  pass. Confirmed the pinned elkjs 0.12.0 behavior directly and checked the
  Java importer and original upstream tests. No runtime change was needed.
- Actual npm tarballs contain both WASM bundles, matching native binary, and
  license/notice files. Isolated native and WASM-only packages passed direct,
  fake-worker, and worker_threads checks on Node.js **18.0.0**, **18.15.0**, and
  **24.19.0** (six checks per runtime).
- Fresh release binaries ran three rounds of the five layered issue scenarios
  (20 measured iterations, three warmups) and eight recursive scenarios
  (five measured iterations, one warmup), with the exact standard input sizes.
  The unchanged baseline **5% gate passed all 13 scenarios** and the recursive
  **60 ms budget passed all eight**. Current raw CSVs and gate reports are under
  `release_0_12_1/gates/`. Existing baselines were not rewritten.

The full synthetic CSVs above remain the primary stable-versus-current
comparison. A later attempted single-scenario `rust_api` repeat was discarded:
that engine's synthetic path ignores `--scenarios` and runs all 26 cases.
It is not included in the performance summary or used to dismiss the recorded
+5.36% comparison against the first patch.
