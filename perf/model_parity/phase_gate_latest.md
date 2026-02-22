# Layered Phase-Gate Summary

- gate_pass: **false**
- base_models(java_status=ok): **1439**
- comparable_models: **1438**
- precheck_errors(비교불가): **1**
- all_match_models: **0**, diverged_models: **1438**

## Precheck

- missing_java_trace: 0
- missing_rust_trace: 1
- missing_both_trace: 0
- missing_compare_entry: 0

### missing_rust_trace_models

- realworld/ptolemy/hierarchical/ptides_powerplant_PowerPlant.elkt

## Phase Gate

| step | processor | reached | match | error |
| ---: | --- | ---: | ---: | ---: |
| 0 | EdgeAndLayerConstraintEdgeReverser | 1438 | 1438 | 0 |
| 1 | GreedyCycleBreaker | 1438 | 1356 | 82 |
| 2 | LayerConstraintPreprocessor | 1356 | 1306 | 50 |
| 3 | NetworkSimplexLayerer | 1306 | 1296 | 10 |
| 4 | LayerConstraintPostprocessor | 1296 | 1295 | 1 |
| 5 | HierarchicalPortConstraintProcessor | 1295 | 1295 | 0 |
| 6 | LongEdgeSplitter | 1295 | 1295 | 0 |
| 7 | PortSideProcessor | 1295 | 1294 | 1 |
| 8 | PortListSorter | 1294 | 1290 | 4 |
| 9 | LayerSweepCrossingMinimizer | 1290 | 1224 | 66 |
| 10 | LayerSweepCrossingMinimizer | 1224 | 897 | 327 |
| 11 | LabelAndNodeSizeProcessor | 897 | 330 | 567 |
| 12 | InnermostNodeMarginCalculator | 330 | 124 | 206 |
| 13 | NetworkSimplexPlacer | 124 | 83 | 41 |
| 14 | LayerSizeAndGraphHeightCalculator | 83 | 30 | 53 |
| 15 | OrthogonalEdgeRouter | 30 | 16 | 14 |
| 16 | OrthogonalEdgeRouter | 16 | 2 | 14 |
| 17 | LabelSideSelector | 2 | 2 | 0 |
| 18 | BKNodePlacer | 2 | 1 | 1 |
| 19 | LayerSizeAndGraphHeightCalculator | 1 | 1 | 0 |
| 20 | OrthogonalEdgeRouter | 1 | 0 | 1 |

## First Failure By Step

- step 1: 82
- step 2: 50
- step 3: 10
- step 4: 1
- step 7: 1
- step 8: 4
- step 9: 66
- step 10: 327
- step 11: 567
- step 12: 206
- step 13: 41
- step 14: 53
- step 15: 14
- step 16: 14
- step 18: 1
- step 20: 1
