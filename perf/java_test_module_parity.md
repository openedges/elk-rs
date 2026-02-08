# Java Test Module Parity Matrix

- java modules scanned: 14
- java test classes total: 188
- java test methods total (`@Test` + `@TestAfterProcessor`): 611
- rust test methods total (`#[test]` in plugins): 886
- direct-mapped java modules: 12
- direct-mapped java tests: 599
- direct-mapped rust tests: 875
- direct-mapped delta (rust - java): 276
- java tests in no-direct modules: 12
- rust modules with tests: 16
- rust-only test modules (not in direct map): 4
- layered issue parity snapshot: status=ok, java=41, rust=41

| java_module | rust_target | mapping | java_classes | java_tests | rust_test_files | rust_tests | delta_rust_minus_java |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| org.eclipse.elk.alg.common.test | plugins/org.eclipse.elk.alg.common | direct | 8 | 37 | 5 | 55 | 18 |
| org.eclipse.elk.alg.disco.test | plugins/org.eclipse.elk.alg.disco | direct | 1 | 3 | 1 | 3 | 0 |
| org.eclipse.elk.alg.force.test | plugins/org.eclipse.elk.alg.force | direct | 1 | 3 | 1 | 3 | 0 |
| org.eclipse.elk.alg.layered.test | plugins/org.eclipse.elk.alg.layered | direct | 96 | 412 | 113 | 540 | 128 |
| org.eclipse.elk.alg.mrtree.test | plugins/org.eclipse.elk.alg.mrtree | direct | 1 | 2 | 1 | 2 | 0 |
| org.eclipse.elk.alg.radial.test | plugins/org.eclipse.elk.alg.radial | direct | 1 | 2 | 2 | 5 | 3 |
| org.eclipse.elk.alg.rectpacking.test | plugins/org.eclipse.elk.alg.rectpacking | direct | 3 | 15 | 4 | 15 | 0 |
| org.eclipse.elk.alg.spore.test | plugins/org.eclipse.elk.alg.spore | direct | 2 | 4 | 2 | 4 | 0 |
| org.eclipse.elk.alg.topdown.test | plugins/org.eclipse.elk.alg.topdownpacking | direct | 3 | 11 | 3 | 11 | 0 |
| org.eclipse.elk.core.test | plugins/org.eclipse.elk.core | direct | 20 | 96 | 43 | 184 | 88 |
| org.eclipse.elk.graph.json.test | plugins/org.eclipse.elk.graph.json | direct | 1 | 7 | 1 | 46 | 39 |
| org.eclipse.elk.graph.test | plugins/org.eclipse.elk.graph | direct | 1 | 7 | 2 | 7 | 0 |
| org.eclipse.elk.alg.test | n/a | no_direct | 46 | 7 | n/a | n/a | n/a |
| org.eclipse.elk.shared.test | n/a | no_direct | 4 | 5 | n/a | n/a | n/a |

## Notes

- `mapping=direct` rows are crate-level structural mapping, not method-level 1:1 semantics proof.
- `org.eclipse.elk.alg.test` and `org.eclipse.elk.shared.test` are treated as no-direct due to architecture mismatch.
- For layered issue method-level parity, use `perf/layered_issue_test_parity.md`.

## Rust-only Test Modules

- plugins/org.eclipse.elk.alg.graphviz.layouter
- plugins/org.eclipse.elk.alg.libavoid
- plugins/org.eclipse.elk.alg.vertiflex
- plugins/org.eclipse.elk.conn.gmf
