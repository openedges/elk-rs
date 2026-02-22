# Drift Phase-Root Summary

- Drift models: **275**
- Source details: `perf/model_parity_full/diff_details.tsv`
- Source manifest: `perf/model_parity_full/rust_manifest.tsv`

| Phase Root | Models | Share | Total Diffs | Avg Diff | Low(1-5) | Medium(6-19) | High(20) | Top Prefixes | Top Categories |
|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| p2_layering | 127 | 46.2% | 2540 | 20.00 | 0 | 0 | 127 | realworld:126, tests:1 | coordinate:125, section:2 |
| p5_edge_routing | 83 | 30.2% | 1611 | 19.41 | 0 | 7 | 76 | realworld:76, tests:5, examples:2 | coordinate:46, section:37 |
| p4_node_placement | 43 | 15.6% | 831 | 19.33 | 1 | 1 | 41 | realworld:38, tests:2, tickets:2 | coordinate:43 |
| p3_crossing_order | 22 | 8.0% | 392 | 17.82 | 0 | 4 | 18 | realworld:22 | coordinate:16, structure:4, section:2 |
