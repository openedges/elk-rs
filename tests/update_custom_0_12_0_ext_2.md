# Custom 0.12.0 ext.2 integration

Date: 2026-10-23. Integrates stable `0.12.1` (`eb586b6d`) into
`custom/0.12.0` (`7d8e9791`) with a merge commit, retaining extension features.
Cargo/npm version: `0.12.0-ext.2`; Java and elkjs baseline: `0.12.0`.
The extension version followed the Java ELK baseline under the policy in effect
for this integration. The merge initially created no tag; a subsequent user
request created annotated `v0.12.0-ext.2` at `3029a99b` and pushed it to `github`,
`origin`, and `gitlab`. No registry package was published; `v0.12.0-ext.1` remains
unchanged. The [policy transition](../VERSIONING.md#1-version-management) approved
later on 2026-10-23 applies only to the next version and preserves this tag,
package name, and validation record.

## Changes and compatibility

The merge includes component hull/1D compaction, zero-size resize fixes,
external-port locking, explicit unsupported-hierarchy errors, JSON allocation
improvements, network-simplex optimizations, and the strict raw trace verifier.

The custom changes relative to stable were compared before and after the merge:
all 14 runtime source-file deltas are unchanged (excluding diff hunk positions).
`ignoreEdgeInLayer`, FIRST/LAST_SEPARATE routing, external-port layer handling,
existing edgeless-port behavior, elk-live, and the extension publication guard
remain in place. Committed expected extension layouts were not changed.

Two test setup problems were corrected during integration:

- The recompute helper added `layoutOptions` and thereby masked legacy
  `properties`, including `hierarchyHandling: INCLUDE_CHILDREN`. It now copies
  those options before adding recompute settings. The existing 18 fixtures run
  with each of five placement strategies and still check retained nodes, ports,
  edge endpoints, finite geometry, and routed sections. Failures name the fixture
  and strategy. Unsupported input checks in the runtime remain enabled.
- Parallel trace-recorder tests could use the same timestamp-based directory
  and delete another test's output. The directory name now also includes the
  process ID and an atomic sequence number.

## Validation

| Check | Result |
|---|---|
| Workspace build and Clippy (`-D warnings`) | Passed; zero warnings |
| Rust workspace tests | 1,107 passed, 0 failed, 2 ignored |
| Python comparator tests | 21 passed |
| JavaScript tests | 49 passed |
| Fresh local NAPI, web WASM, Node WASM builds | Passed (development profiles) |
| Committed extension fixtures | 18/18 on each of the three bindings |
| Java JSON parity | 550/550 each on NAPI and web WASM; no missing baseline |
| Recompute coverage | 18 fixtures × 5 strategies, plus endpoint/unzipping checks |
| Version synchronization | All manifests and internal pins `0.12.0-ext.2` |
| Static metadata/phase wiring | 11/12 strict checks passed; one existing extension-option difference |

Static option support reports layered Rust154/Java153 because
`org.eclipse.elk.layered.layering.ignoreEdgeInLayer` is a custom option.
The default-mode checker also reports that extra pair informationally; it has
no default mismatch and passes. No ignore rule or baseline was added.

### Full raw intermediate-state and final-layout comparison

The corrected Java 0.12.0 reference includes the three documented patches:
self-loop determinism, finite zero-size resize, and zero-height scanline ordering.
Its existing generated corpus was reused because the Java/model pins and patches
did not change. Rust layouts and raw traces were regenerated for this merge.
Both sides use root layered layout and seed 1, preserving nested algorithms.

- All 1,998 input outcomes are accounted for, with no missing models.
- 1,997 successful layouts: 1,996 final layouts match Java; one differs.
- 1,995 successful models match both final output and every intermediate state.
- Ticket 588 raises the same `UnsupportedGraphException` on both sides: this
  negative test **passes**, and is included in the 1,998-case denominator.
- 196,407 states were compared: 196,373 match and 34 differ, confined to the
  two pre-existing custom cases below.
- The unmodified strict verifier exits **1**, reporting two differing models.
  This is not a claim of complete Java parity. No differences were suppressed.

| Existing custom case | Final output | Raw trace |
|---|---|---|
| `emptyLayerFirst.elkt` | Four field differences | 17/19 states differ |
| `emptyLayerLast.elkt` | Matches | 17/19 states differ |

Both first diverge at step 0, `EdgeAndLayerConstraintEdgeReverser`: the extension
allows EAST-to-WEST reversal between normal FIRST/LAST_SEPARATE nodes.
The stricter topology comparator now detects this earlier than the old report,
which first reported geometry differences at steps 5 and 6. The custom source
changes and both final outputs are unchanged from the previous custom tip.
See [compact evidence](update_custom_0_12_0_ext_2.json) for field-level examples.

### Ordinary-algorithm regression against the previous custom tip

A separate run preserves each model's original algorithm. Of 1,998 inputs,
1,996 successful outputs are unchanged from the previous custom baseline.
Only the two stable fixes change behavior:

- Ticket 213 now uses the actual component compaction algorithm. It matches
  the corrected Java reference in final output and all 229 intermediate states.
- Ticket 588 now rejects the unsupported cross-hierarchy edge with
  `UnsupportedGraphException`, instead of silently accepting incomplete layout.

The ordinary run must not be compared to the root-forced layered run: three
interactive/rectpacking examples intentionally select a different root algorithm
in those two modes.

## Reproduction and limits

Run `cargo build --workspace`, `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, and
`python3 -B -m unittest discover -s scripts/tests -p 'test_*.py'`.
Follow [the full trace workflow](phase_trace_audit.md#reproduce) to generate
both references and invoke `scripts/verify_phase_corpus.py` without waivers.
The custom branch retains the two documented divergences above. For JSON parity,
use the explicit freshly built backend commands in [TESTING.md](../TESTING.md).

Local raw artifacts are under `/tmp/elk-custom-ext2`; compact counts, input and
manifest hashes, expected-error outcomes, and differing cases are committed in
[the JSON evidence](update_custom_0_12_0_ext_2.json). The historical stable report
is [release_0_12_1.md](release_0_12_1.md), and the previous custom report is
[upgrade_custom_0_12_0.md](upgrade_custom_0_12_0.md).

Performance was not remeasured for this merge. Stable 0.12.1 measurements are
historical evidence, not a new measurement of the custom branch. No cross-platform
release artifacts or registry publication were requested for this branch update.
