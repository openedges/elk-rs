# ELK Model Parity Report

- manifest: `/Users/cody.ij.hwang/Projects/github/elk-rs/perf/model_parity_tests/rust_manifest.tsv`
- total rows: 193
- compared rows: 185
- matched rows: 51
- drift rows: 134
- skipped rows (java/rust non-ok): 8
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 2301

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| section | 990 | 43.0% |
| coordinate | 938 | 40.8% |
| label | 175 | 7.6% |
| structure | 90 | 3.9% |
| ordering | 65 | 2.8% |
| other | 43 | 1.9% |

### Top Diff Path Prefixes

- `children[*]/y`: 468 (20.3%)
- `edges[*]/sections[*]/endPoint`: 274 (11.9%)
- `children[*]/edges[*]/sections[*]`: 265 (11.5%)
- `edges[*]/sections[*]/bendPoints[*]`: 241 (10.5%)
- `edges[*]/sections[*]/startPoint`: 235 (10.2%)
- `children[*]/x`: 150 (6.5%)
- `edges[*]/labels[*]/y`: 119 (5.2%)
- `children[*]/ports[*]/x`: 88 (3.8%)
- `children[*]/children[*]/x`: 71 (3.1%)
- `children[*]/children[*]/y`: 67 (2.9%)

## Drift Samples

- `tests/core/label_placement/port_labels/next_to_port_if_possible_inside.elkt`: diffs=10 [coordinate=8, other=2], first: children[0]/height: number mismatch (64.0 != 124.0)
- `tests/core/label_placement/port_labels/treat_as_group_outside.elkt`: diffs=4 [label=2, coordinate=1, other=1], first: children[0]/height: number mismatch (30.0 != 20.0)
- `tests/core/label_placement/port_labels/variants.elkt`: diffs=20 [coordinate=13, section=4, label=3], first: children[0]/children[0]/x: number mismatch (12.0 != 36.0)
- `tests/core/node_size/inside_port_labels.elkt`: diffs=16 [coordinate=14, label=1, other=1], first: children[0]/height: number mismatch (64.0 != 0.0)
- `tests/layered/compaction_oned/hierarchical_ports/hports1.elkt`: diffs=20 [section=12, coordinate=4, label=4], first: children[0]/children[0]/x: number mismatch (32.0 != 52.0)
- `tests/layered/compaction_oned/labels/edgeLabelAndSplines.elkt`: diffs=20 [section=8, coordinate=6, label=4, ordering=2], first: children[0]/y: number mismatch (20.5 != 19.0)
- `tests/layered/compaction_oned/labels/edgeLabelShouldBeCentered.elkt`: diffs=12 [section=6, coordinate=3, label=2, ordering=1], first: children[0]/y: number mismatch (32.66666666666667 != 15.333333333333334)
- `tests/layered/compaction_oned/labels/edgeLabelSideSelection01.elkt`: diffs=16 [section=8, coordinate=4, label=3, other=1], first: children[0]/y: number mismatch (15.0 != 12.0)
- `tests/layered/compaction_oned/labels/labels1.elkt`: diffs=20 [coordinate=12, section=4, label=3, ordering=1], first: children[0]/x: number mismatch (305.0 != 340.0)
- `tests/layered/compaction_oned/labels/labels2.elkt`: diffs=20 [coordinate=9, section=6, label=4, ordering=1], first: children[0]/x: number mismatch (554.0 != 1021.0)
- `tests/layered/compaction_oned/nsport/south_port.elkt`: diffs=4 [section=2, coordinate=1, other=1], first: children[2]/x: number mismatch (41.3333 != 61.3333)
- `tests/layered/compaction_oned/selfloop/selfloop_crash.elkt`: diffs=20 [section=12, coordinate=6, ordering=2], first: children[0]/y: number mismatch (73.0 != 72.0)
- `tests/layered/compaction_oned/selfloop/selfloop_orthogonal.elkt`: diffs=16 [section=10, coordinate=4, structure=2], first: children[0]/y: number mismatch (42.0 != 12.0)
- `tests/layered/compaction_oned/selfloop/selfloop_spline.elkt`: diffs=20 [section=11, coordinate=6, ordering=3], first: children[0]/y: number mismatch (43.0 != 12.0)
- `tests/layered/compaction_oned/splines/moreThanOneStraightSegmentPath.elkt`: diffs=20 [coordinate=15, section=5], first: children[0]/y: number mismatch (77.5 != 72.5)
- `tests/layered/compaction_oned/splines/moreThanOneStraightSegmentPath2.elkt`: diffs=20 [section=11, coordinate=9], first: children[1]/x: number mismatch (52.0 != 32.0)
- `tests/layered/connected_components/all_four_port_sides.elkt`: diffs=20 [section=12, coordinate=4, structure=4], first: children[0]/children[1]/x: number mismatch (62.0 != 12.0)
- `tests/layered/connected_components/compound02.elkt`: diffs=20 [coordinate=16, section=4], first: children[0]/children[0]/x: number mismatch (12.0 != 104.5)
- `tests/layered/connected_components/compound03.elkt`: diffs=20 [coordinate=20], first: children[0]/children[0]/x: number mismatch (12.0 != 362.0)
- `tests/layered/connected_components/compound04.elkt`: diffs=20 [section=12, coordinate=5, structure=3], first: children[0]/children[0]/x: number mismatch (12.0 != 82.0)
