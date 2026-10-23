# Extension version 0.12.1-ext.1

Date: 2026-10-23. Branch: `custom/0.12.0`.
Package version: `0.12.1-ext.1`. Annotated tag: `v0.12.1-ext.1`.
Stable base: `0.12.1` (`eb586b6d`); Java/elkjs reference: `0.12.0`.

## Version transition

This version applies the approved stable-base naming policy. It updates the
Cargo/npm manifests, lockfiles, and exact internal dependency pins from
`0.12.0-ext.2` to `0.12.1-ext.1`. It has its own version commit and tag.
The previous `v0.12.0-ext.2` remains at
`3029a99bf546e9ca9d54f4a21827e4d9c059d64c`; all earlier tags are preserved.
The branch name and Java/elkjs compatibility pins do not change.

Source review against that previous tag confirms:

- All runtime sources, tests, fixtures, scripts, workflows, and submodule pins
  are unchanged. The remaining changes are documentation and version metadata.
- The 13 changed manifest/lock files are structurally identical after replacing
  the old extension version with the new one. TOML and JSON parsing verifies
  that no third-party dependency or unrelated manifest field changed.
- All 20 local Cargo lockfile package versions and 17 exact workspace dependency
  pins use the new version. The six native npm package pins agree with it.

## Checks run for this version

| Check | Result |
|---|---|
| `python3 scripts/bump_version.py --check` | `0.12.1-ext.1` |
| `cargo build --workspace` | Passed; zero warnings/errors |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed; zero warnings |
| `cargo test --workspace` | 1,107 passed, 0 failed, 2 ignored |
| Python comparator unit tests | 21 passed |
| Local macOS arm64 NAPI development build | Passed at the new package version |
| JavaScript suite (`npm test`) | 49 passed across 14 test files |
| Manifest/dependency/source review and `git diff --check` | Passed |

The JavaScript suite uses the rebuilt local NAPI addon for native loading.
Its WASM fallback checks reuse the previous extension's local WASM artifacts,
whose runtime sources are identical; these are not new-version WASM builds.
The tag workflow builds fresh web/Node WASM and six platform NAPI artifacts.
Expected invalid-input exceptions are asserted by passing tests, not counted
as test failures. The two ignored Rust tests remain ignored as before.

No tests were added because this change introduces no runtime behavior.
Rust formatting was not rerun: no Rust source changed, and the pre-existing
workspace-wide formatting differences remain outside this version transition.

## Inherited full-corpus evidence and limits

The full Java layout and raw phase-state comparison was not regenerated for
this metadata-only change. The runtime, test data, comparator, and reference
pins are identical to the previously validated tag. The original evidence is
preserved in [the ext.2 integration report](update_custom_0_12_0_ext_2.md) and
[its machine-readable results](update_custom_0_12_0_ext_2.json).
These results are inherited evidence, not a new execution:

- 1,998 input outcomes: 1,997 successful layouts and one passing assertion that
  both implementations raise the same `UnsupportedGraphException` (ticket 588).
- 1,996/1,997 final layouts match; 1,995/1,997 successful models match both the
  final layout and all raw intermediate states.
- 196,407 states compared: 196,373 match and 34 differ in the retained custom
  `emptyLayerFirst.elkt` / `emptyLayerLast.elkt` cases. The strict verifier
  exits **1**; this version does not claim complete Java equivalence.
- The Java reference is 0.12.0 plus the three documented corrections for
  self-loop determinism, zero-size resize, and zero-height scanline ordering.
  It is not unmodified Java 0.12.0.
- The previous binding checks passed 18 extension fixtures on all three
  bindings and 550/550 Java JSON cases each on NAPI and web WASM.

Performance was not remeasured. Historical stable measurements are not new
measurements of this extension version.

## Publication scope

Push the version commit on `custom/0.12.0` and annotated `v0.12.1-ext.1` to
`github`, `origin`, and `gitlab`. Preserve existing branch families and tags.
The existing extension-tag workflow builds artifacts, but skips npm/crates.io
publication and GitHub Release creation. No workflow or publication setting
is changed. Remote workflow outcomes are separate from the local checks above.

For the next revision on stable `0.12.1`, use `0.12.1-ext.2`; incorporating
stable `0.12.2` starts at `0.12.2-ext.1`. See [VERSIONING.md](../VERSIONING.md).
