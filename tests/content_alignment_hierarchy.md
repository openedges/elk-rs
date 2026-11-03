# Content alignment with a minimum size under INCLUDE_CHILDREN

Date: 2026-11-03. Fix for [elk-rs issue #3](https://github.com/openedges/elk-rs/issues/3)
on `fix/content-alignment-offset`.
Reference: ELK v0.12.0 (`external/elk` at `32331291`), elkjs 0.12.0 (`external/elkjs` at `ff5771d7`).

## Defect

`ElkGraphLayoutTransferrer::apply_layout` called a Rust-only step,
`apply_content_alignment_offset`, before writing the layered graph back to the
ElkGraph. The step compared the untransposed `nodeSize.minimum` of a compound node
with the graph size already in input orientation and shifted the whole content by
the apparent spare room. Under `RIGHT` and `LEFT` the graph had already been resized
to its minimum, so the difference was zero and nothing moved. Under `DOWN` and `UP`
the two sizes live in different orientations, so the step saw room that does not
exist and moved the children outside their parent. Edges wired to the children moved
with them and no longer reached the parent's ports.

Java 0.12.0 `ElkGraphLayoutTransferrer.applyLayout` has no such step. Content
alignment is applied only by `ElkLayered.resizeGraphNoReallyIMeanIt` and
`HierarchicalNodeResizingProcessor`, which elk-rs already ports. The step was
introduced in `e182e764` (2026-02-15) during early parity work and never matched
upstream.

## Fix

The call and the function were removed. No other code changed.

## Reference cases

`tests/content_alignment_hierarchy/*.input.json` are the reporter's graph and the
variations listed in the issue: a compound node with `MINIMUM_SIZE`,
`nodeSize.minimum`, and a content alignment, containing one 100 x 40 child, under
`INCLUDE_CHILDREN`.

| Case | Direction | Minimum | Alignment | Notes |
|---|---|---|---|---|
| `down_h_center` | DOWN | (300, 40) | H_CENTER V_TOP | reporter's graph |
| `up_h_center` | UP | (300, 40) | H_CENTER V_TOP | |
| `down_v_center` | DOWN | (40, 300) | H_LEFT V_CENTER | vertical variant |
| `down_h_left` | DOWN | (300, 40) | H_LEFT V_TOP | unaffected control |
| `down_nested_h_center` | DOWN | (300, 40) | H_CENTER V_TOP | one level deeper |
| `down_ports_h_center` | DOWN | (300, 40) | H_CENTER V_TOP | parent port wired to child port |
| `down_separate_children` | DOWN | (300, 40) | H_CENTER V_TOP | no `INCLUDE_CHILDREN` |
| `right_h_center` | RIGHT | (300, 40) | H_CENTER V_TOP | unaffected control |
| `left_h_center` | LEFT | (300, 40) | H_CENTER V_TOP | unaffected control |
| `default_direction_h_center` | none | (300, 40) | H_CENTER V_TOP | unaffected control |

`*.expected.json` are the elkjs 0.12.0 layouts of the same inputs with the GWT-only
`$H` fields removed. elkjs is compiled from the pinned Java sources, and the issue
reports that Java ELK 0.12.0 produces the same coordinates for the reporter's graph.
The same ten inputs were also laid out with the compiled Java ELK 0.12.0 reference
build (`scripts/java_model_parity_trace.sh`, JDK 21, Maven 3.9.16, isolated worktree
with the three standard reference patches, none of which touch these graphs). All ten
Java layouts equal the stored expected files to 1e-6.

The minimum size is applied in the layered graph's internal orientation, as in Java.
Under `DOWN` the parent therefore becomes 124 x 300 and `H_CENTER` centers the child
along the 300 axis at (12, 130). This is the upstream behavior and is kept as is.

Before the fix, the five `DOWN`/`UP` cases with an alignment on the transposed axis
placed the child at x = 100 (or y = 130 for `down_v_center`), outside the 124-wide
parent. After the fix all ten cases match the reference exactly.

## Verification

- `cargo test -p org-eclipse-elk-graph-json --test content_alignment_hierarchy`:
  ten reference cases, a containment check for every case, and the issue's coordinates.
- The same test run against unmodified `main` (`eb586b6d`) in a throwaway worktree
  fails seven of the twelve tests (the five transposed-axis cases, the containment
  check and the issue coordinates) and passes the five controls.
- Workspace build and Clippy: zero warnings. `cargo test --workspace`: 1,093 passed,
  0 failed, 2 ignored (1,081 before plus the twelve new tests).
- Model parity against a fresh Java export (JDK 21, Maven 3.9.16, standard reference
  patches): 1,998 models, 1,989 compared, 1,989 matched, 0 drift, 9 skipped without
  Java output (the known label-stacking, north-south-port and 588 cases), 0 errors.
- Strict full phase-step trace (`run_full_trace_parity.sh`, corrected Java reference,
  JDK 21): 1,998 inputs, 1,997 successful layouts, one expected error (588),
  196,407 intermediate states verified, 0 failed models, `status: pass`. Identical to
  the 0.12.1 baseline, as expected from the coverage note below.

## Coverage note

None of the 1,998 corpus models combines `contentAlignment` with
`INCLUDE_CHILDREN`, so the model parity and phase trace gates could not detect
this defect and are not expected to change with the fix. The reference cases above
close that gap for this option combination.
