# Layered Phase-Gate Summary

- gate_pass: **false**
- base_models(java_status=ok): **1439**
- comparable_models: **1439**
- precheck_errors(비교불가): **0**
- all_match_models: **0**, diverged_models: **1439**

## Precheck

- missing_java_trace: 0
- missing_rust_trace: 0
- missing_both_trace: 0
- missing_compare_entry: 0

## Phase Gate

| step | processor | reached | match | error |
| ---: | --- | ---: | ---: | ---: |
| 0 | EdgeAndLayerConstraintEdgeReverser | 1439 | 1439 | 0 |
| 1 | GreedyCycleBreaker | 1439 | 1439 | 0 |
| 2 | LayerConstraintPreprocessor | 1439 | 1439 | 0 |
| 3 | NetworkSimplexLayerer | 1439 | 1439 | 0 |
| 4 | LayerConstraintPostprocessor | 1439 | 1439 | 0 |
| 5 | HierarchicalPortConstraintProcessor | 1439 | 1439 | 0 |
| 6 | LongEdgeSplitter | 1439 | 1439 | 0 |
| 7 | PortSideProcessor | 1439 | 1439 | 0 |
| 8 | PortListSorter | 1439 | 1398 | 41 |
| 9 | LayerSweepCrossingMinimizer | 1398 | 1327 | 71 |
| 10 | LayerSweepCrossingMinimizer | 1327 | 1011 | 316 |
| 11 | LayerSweepCrossingMinimizer | 1011 | 396 | 615 |
| 12 | InnermostNodeMarginCalculator | 396 | 158 | 238 |
| 13 | NetworkSimplexPlacer | 158 | 144 | 14 |
| 14 | LayerSizeAndGraphHeightCalculator | 144 | 143 | 1 |
| 15 | OrthogonalEdgeRouter | 143 | 128 | 15 |
| 16 | OrthogonalEdgeRouter | 128 | 79 | 49 |
| 17 | OrthogonalEdgeRouter | 79 | 26 | 53 |
| 18 | BKNodePlacer | 26 | 26 | 0 |
| 19 | LayerSizeAndGraphHeightCalculator | 26 | 21 | 5 |
| 20 | OrthogonalEdgeRouter | 21 | 2 | 19 |
| 21 | OrthogonalEdgeRouter | 2 | 1 | 1 |
| 22 | HierarchicalPortOrthogonalEdgeRouter | 1 | 1 | 0 |
| 23 | LongEdgeJoiner | 1 | 1 | 0 |
| 24 | NorthSouthPortPostprocessor | 1 | 1 | 0 |
| 25 | EndLabelSorter | 1 | 1 | 0 |
| 26 | ReversedEdgeRestorer | 1 | 1 | 0 |
| 27 | EdgeAndLayerConstraintEdgeReverser | 1 | 1 | 0 |
| 28 | GreedyCycleBreaker | 1 | 1 | 0 |
| 29 | LayerConstraintPreprocessor | 1 | 1 | 0 |
| 30 | NetworkSimplexLayerer | 1 | 1 | 0 |
| 31 | LayerConstraintPostprocessor | 1 | 1 | 0 |
| 32 | LongEdgeSplitter | 1 | 1 | 0 |
| 33 | PortSideProcessor | 1 | 1 | 0 |
| 34 | InvertedPortProcessor | 1 | 1 | 0 |
| 35 | PortListSorter | 1 | 1 | 0 |
| 36 | LayerSweepCrossingMinimizer | 1 | 1 | 0 |
| 37 | LayerSweepCrossingMinimizer | 1 | 0 | 1 |

## First Failure By Step

- step 8: 41
- step 9: 71
- step 10: 316
- step 11: 615
- step 12: 238
- step 13: 14
- step 14: 1
- step 15: 15
- step 16: 49
- step 17: 53
- step 19: 5
- step 20: 19
- step 21: 1
- step 37: 1
