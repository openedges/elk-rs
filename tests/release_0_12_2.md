# elk-rs 0.12.2 Release Validation

Date: 2026-11-03. Package version: **0.12.2**. Java/elkjs reference: **0.12.0**.
The runtime is `main` after [PR #4](https://github.com/openedges/elk-rs/pull/4)
(merge `a888bd54`; fix commit `57833424`) plus the version-only bump on
`release/0.12.2`. The package patch changes no upstream pins or third-party
dependency versions. The bump merges through a PR into `main`, then advances the
existing `releases/0.12.0` line and receives annotated tag `v0.12.2`.

## Change

One fix, [issue #3](https://github.com/openedges/elk-rs/issues/3): under
`INCLUDE_CHILDREN` with direction `DOWN` or `UP`, a compound node with a minimum
size and a content alignment placed its children outside itself. A Rust-only
alignment offset in `ElkGraphLayoutTransferrer`, absent from Java, was removed.
Root cause, reference provenance and the pre-fix failure evidence are in
[the reference cases](content_alignment_hierarchy.md).

## Correctness

- Workspace build, Clippy (`--all-targets`) and release build: zero warnings.
- Workspace tests after the bump: **1,092 passed, zero failed, two ignored**
  (1,081 before plus the eleven new reference tests).
- Model parity against a fresh Java export (JDK 21, Maven 3.9.16, the three standard
  reference patches): **1,998 models, 1,989 compared, 1,989 matched, 0 drift,
  9 skipped without Java output, 0 errors**.
- Strict full phase-step gate (`run_full_trace_parity.sh`, corrected Java reference):
  **1,998 inputs, 1,997 matching layouts plus one matching exception,
  196,407 intermediate states verified, 0 failed models, `status: pass`**.
  Both corpus gates ran on the fix commit's tree, which differs from this release
  only by version strings. No corpus model combines `contentAlignment` with
  `INCLUDE_CHILDREN`, so these results are unchanged from 0.12.1 by design; the
  ten new reference cases cover that combination and match both elkjs 0.12.0 and
  the compiled Java 0.12.0 reference.
- Metadata gate, strict mode, against the Tycho-generated Java sources:
  phase wiring plus the eleven option/algorithm checks all report `status: ok`
  (12/12). The regenerated reports count one more layered option and wiring row on
  both sides than the previously committed reports; Java and Rust agree.

## JavaScript package

- `sh build.sh` produced the browser ESM and Node.js CommonJS WASM bundles
  (wasm-pack 0.13.1). The `darwin-arm64` native addon was built with
  `@napi-rs/cli` 3.10.10 invoked directly; `build.sh`'s `npx napi` step does not
  find a CLI installed outside the project. The other five platforms are built by
  `release.yml`.
- Vitest: **49 passed** (14 files) against the rebuilt WASM and native addon.
- Three-way JSON parity (`test/parity/run.mjs --require-java`) over the 550 JSON
  models with the fresh Java baseline: native and WASM backends each
  **550/550 Java matches, 530 `PASS` + 20 `ELKJS_DRIFT`, 0 failures**.
- `npm pack --dry-run`: 20 files, 3.4 MB packed, both WASM bundles, `LICENSE.md`,
  `NOTICE` and typings present. `publish_npm.sh --dry-run --expect-version 0.12.2`
  reported all seven packages. `publish_crates.py --dry-run --allow-dirty`
  packaged and verified all 18 crates.

## Performance

MacBook Pro `Mac15,6`, Apple M3 Pro (11 cores: 5P+6E), 36 GiB RAM, macOS 27.2,
Rust 1.97.1, release builds (`scripts/run_parity_*.sh`), unchanged baselines.

Required gate, `PARITY_COMPARE_MODE=baseline sh scripts/check_parity_regression.sh 5 3`
(three measured rounds per scenario, 13 scenarios) and the recursive 60 ms budget:

- First run, started right after the Java/NAPI/WASM builds with a 5-minute load
  average of 5.7: three sub-2 ms scenarios exceeded the 5% threshold
  (`box_sparse` +26.2%, `issue_405` +15.6%, `issue_603` +24.4%) while the other ten
  were 7% to 47% faster than baseline; individual rounds varied up to 4x
  (`box_sparse` 0.27 / 1.09 / 0.34 ms). Recorded as
  `release_0_12_2/gates/regression-run1-busy.txt` with its CSVs.
- Second run, after a one-minute settle at load average 1.6: **13/13 scenarios
  within the 5% gate**, every scenario 3% slower to 47% faster than baseline
  (`fixed_dense` +3.33% is the only increase). Recorded as
  `release_0_12_2/gates/regression.txt`, `recursive.csv`, `layered-issues.csv`.
- Recursive runtime budget: **8/8 scenarios under 60 ms** in both runs
  (`runtime-budget.md`; the slowest scenario averages 1.9 ms).

The change removes one function call per transferred graph and adds no work, so
the first run's exceedances have no code-level mechanism. The same-machine A/B
below confirms this directly.

Same-machine alternating A/B of the 26 synthetic scenarios, `perf_benchmark`
release builds of `v0.12.1` (`eb586b6d`) and this branch, three rounds with the
order alternated and reversed, 3 warmups and 10 iterations per scenario, median of
the per-round means per scenario, no builds or tests running concurrently
(load average 1.8 at the end). Binary hashes, raw CSV rounds and the summary are
under `release_0_12_2/perf/`.

| API | v0.12.1 ms | 0.12.2 ms | Change | Largest increase | Largest decrease |
|---|---:|---:|---:|---|---|
| Native graph API | 22.8229 | 22.9003 | +0.34% | `force_xlarge` +2.84% | `layered_large` -1.66% |
| JSON API | 24.4828 | 24.3246 | -0.65% | `force_medium` +1.81% | `stress_xlarge` -2.39% |

All 52 scenario comparisons are within 5%; none of the layered scenarios that
exercise the changed transferrer moved by more than 1.7%. The aggregate is the
arithmetic mean across scenarios, not application latency, and these short
timings carry noise; they show no regression rather than a speedup.

A first A/B attempt reused the thirteen sub-2 ms gate scenarios with their default
5/20 iterations; it produced deltas from -53% to +88% in both directions within the
same run (CSVs under `gates/ab-small-scenarios/`) and is recorded as inconclusive
measurement noise, which is why the 26-scenario benchmark above was used instead.

## Release steps

1. `python3 scripts/bump_version.py 0.12.2`; `--check` prints `0.12.2`.
2. Commit `release: prepare elk-rs 0.12.2` on `release/0.12.2`, PR into `main`,
   CI green, merge.
3. Advance `releases/0.12.0` to the merge commit; annotated tag `v0.12.2`;
   push branch and tag. `release.yml` builds the six NAPI binaries and the WASM
   bundles, publishes the missing npm/crates.io versions through Trusted
   Publishing, and attaches the assets to the GitHub Release.
Publication results (workflow run, registry versions, release assets) are appended
after the tag run completes.
