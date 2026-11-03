# Extension version 0.12.2-ext.1

Date: 2026-11-03. Branch: `custom/0.12.0`.
Package version: `0.12.2-ext.1`. Annotated tag: `v0.12.2-ext.1` (build only).
Stable base: `0.12.2` (`a7f40057`, tag `v0.12.2`, published 2026-11-03);
Java/elkjs reference: `0.12.0`.

## Integration

Merge commit `b966a21c` brings `main` at `be2f0770` into the extension line on
top of `78ffb7c8` (`v0.12.1-ext.1`). Relative to stable `0.12.1` this adds:

- The [issue #3](https://github.com/openedges/elk-rs/issues/3) fix: the Rust-only
  `apply_content_alignment_offset` step was removed from the layered
  `ElkGraphLayoutTransferrer`, so children of a compound node with a minimum size
  and a content alignment stay inside it under `INCLUDE_CHILDREN` with `DOWN`/`UP`.
  Its ten reference cases (`tests/content_alignment_hierarchy/`, elkjs 0.12.0
  layouts confirmed against compiled Java 0.12.0) come along.
- The 0.12.2 release validation, the publication records, and the explicit
  `releases/<ELK base version>` branch rule.

Runtime sources merged without conflicts; the custom features (ignoreEdgeInLayer,
in-layer edge routing, edgeless port layout fix, elk-live) are unchanged. Conflicts
were confined to version manifests and lockfiles (resolved by keeping the extension
version, then `bump_version.py 0.12.2-ext.1`), two regenerated metadata reports, and
five documents whose stable and custom records were combined.

## Checks run for this version

| Check | Result |
|---|---|
| `python3 scripts/bump_version.py --check` | `0.12.2-ext.1` |
| `cargo build --workspace` | Passed; zero warnings/errors |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed; zero warnings |
| `cargo test --workspace` | **1,118 passed, 0 failed, 2 ignored** (1,107 before plus the eleven stable reference tests) |
| Python comparator unit tests | 21 passed |
| Strict metadata gate against the Tycho-generated Java sources | 11/12 `status: ok`; `algorithm_option_support` keeps the known extension difference (layered options Java 153 / Rust 154 from `ignoreEdgeInLayer`) |
| Fresh web + Node.js WASM bundles (wasm-pack 0.13.1) and macOS arm64 NAPI (`@napi-rs/cli` 3.10.10) | Built at the new version |
| JavaScript suite (`npm test`) | 49 passed across 14 test files |
| Java JSON parity, 550 models, fresh Java 0.12.0 baseline | native and WASM backends each **550/550 Java matches**, 530 `PASS` + 20 `ELKJS_DRIFT`, 0 failures, 0 missing baselines |
| Model parity, 1,998 models, Rust vs. the fresh Java 0.12.0 baseline | 1,989 compared, **1,988 matched, 1 drift** (`tests/layered/layer_constraints/emptyLayerFirst.elkt`, the retained custom difference), 9 skipped without Java output, 0 errors |
| Strict full phase trace, corrected Java reference | 1,998 inputs, 1,997 successful layouts + 1 matching exception, **196,407 states verified; 2 models differ** (`emptyLayerFirst.elkt`: 17/19 drift steps and 4 final-layout differences; `emptyLayerLast.elkt`: 17/19 drift steps, final layout equal), verifier exit 1 as before |

## Known differences, unchanged

The strict corpus result is identical to the custom line before this merge
([ext.2 integration report](update_custom_0_12_0_ext_2.md)): 1,996/1,997 final
layouts match the corrected Java 0.12.0 reference, 1,995/1,997 models match every
raw intermediate state, and the 34 differing states are confined to the two
retained `tests/layered/layer_constraints/emptyLayer{First,Last}.elkt` cases, where
the custom in-layer edge handling diverges from Java by design. The stable issue #3
fix changed no other model, as the stable gate predicted (no corpus model combines
`contentAlignment` with `INCLUDE_CHILDREN`). This version therefore does not claim
complete Java equivalence; it claims the same, documented divergence as before.
Compact machine-readable results: [release_0_12_2_ext_1.json](release_0_12_2_ext_1.json).

Performance was not remeasured for this extension version. The stable 0.12.2
release measured the same runtime change against `v0.12.1` on this machine
(native +0.34%, JSON API -0.65%, all 52 synthetic comparisons within 5%; see
[the stable report](release_0_12_2.md)); the custom features did not change.

## Remote results

- Tag run [41613773967](https://github.com/openedges/elk-rs/actions/runs/41613773967) on
  `v0.12.2-ext.1`: preflight, WASM bundle and all six NAPI targets succeeded; the
  crates.io, npm, dry-run and GitHub Release jobs were skipped as designed for `-ext.`
  tags. CI on `custom/0.12.0` ([41613771070](https://github.com/openedges/elk-rs/actions/runs/41613771070))
  passed.

## Publication scope

Push the merge and version commits on `custom/0.12.0` and annotated
`v0.12.2-ext.1` to `origin`. The extension-tag workflow builds the artifacts and
skips npm/crates.io publication and GitHub Release creation. Existing tags and
branch families are preserved. For another revision on stable `0.12.2`, use
`0.12.2-ext.2`; a later stable patch starts at `-ext.1` again.
