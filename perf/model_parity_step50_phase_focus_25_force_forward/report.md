# ELK Model Parity Report

- manifest: `perf/model_parity_step50_phase_focus_25_force_forward/rust_manifest.tsv`
- total rows: 25
- compared rows: 25
- matched rows: 16
- drift rows: 9
- skipped rows (java/rust non-ok): 0
- compare errors: 0
- abs tolerance: 1e-06
- max diffs per model: 20
- total diffs across all models: 180

## Drift Classification Summary

| Category | Count | Percentage |
|----------|------:|-----------:|
| coordinate | 88 | 48.9% |
| section | 70 | 38.9% |
| structure | 18 | 10.0% |
| ordering | 4 | 2.2% |

### Top Diff Path Prefixes

- `children[*]/edges[*]/sections[*]`: 56 (31.1%)
- `children[*]/children[*]/y`: 50 (27.8%)
- `children[*]/y`: 38 (21.1%)
- `edges[*]/sections[*]/bendPoints[*]`: 12 (6.7%)
- `edges[*]/sections[*]/endPoint`: 6 (3.3%)
- `children[*]/edges[*]`: 4 (2.2%)
- `edges[*]`: 4 (2.2%)
- `edges[*]/sections[*]`: 4 (2.2%)
- `children[*]/edges[*]/junctionPoints`: 4 (2.2%)
- `edges[*]/sections[*]/startPoint`: 2 (1.1%)

## Drift Samples

- `tests/layered/compaction_oned/selfloop/selfloop_spline.elkt`: diffs=20 [section=16, coordinate=4], first: children[0]/y: number mismatch (43.0 != 12.0)
- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkg`: diffs=20 [section=11, coordinate=5, structure=4], first: children[2]/children[0]/y: number mismatch (272.4 != 254.4)
- `realworld/ptolemy/hierarchical/continuous_cartpendulum_CartPendulum.elkt`: diffs=20 [section=11, coordinate=5, structure=4], first: children[2]/children[0]/y: number mismatch (272.4 != 254.4)
- `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkg`: diffs=20 [coordinate=14, structure=4, section=2], first: children[1]/y: number mismatch (313.0 != 151.0)
- `realworld/ptolemy/flattened/ptolemy_brewery_Brewery.elkt`: diffs=20 [coordinate=14, structure=4, section=2], first: children[1]/y: number mismatch (313.0 != 151.0)
- `realworld/ptolemy/hierarchical/ptolemy_brewery_Brewery.elkg`: diffs=20 [coordinate=10, section=9, ordering=1], first: children[0]/children[0]/y: number mismatch (178.0 != 180.5)
- `realworld/ptolemy/hierarchical/ptolemy_brewery_Brewery.elkt`: diffs=20 [coordinate=10, section=9, ordering=1], first: children[0]/children[0]/y: number mismatch (178.0 != 180.5)
- `realworld/ptolemy/hierarchical/ptolemy_brewery_BreweryWireless.elkg`: diffs=20 [coordinate=13, section=5, structure=1, ordering=1], first: children[0]/y: number mismatch (64.0 != 212.5)
- `realworld/ptolemy/hierarchical/ptolemy_brewery_BreweryWireless.elkt`: diffs=20 [coordinate=13, section=5, structure=1, ordering=1], first: children[0]/y: number mismatch (64.0 != 212.5)
