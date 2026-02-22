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
| 0 | EdgeAndLayerConstraintEdgeReverser | 1439 | 389 | 1050 |
| 1 | EdgeAndLayerConstraintEdgeReverser | 389 | 389 | 0 |
| 2 | GreedyCycleBreaker | 389 | 389 | 0 |
| 3 | LayerConstraintPreprocessor | 389 | 387 | 2 |
| 4 | NetworkSimplexLayerer | 387 | 387 | 0 |
| 5 | LayerConstraintPostprocessor | 387 | 387 | 0 |
| 6 | LongEdgeSplitter | 387 | 387 | 0 |
| 7 | PortSideProcessor | 387 | 387 | 0 |
| 8 | PortListSorter | 387 | 387 | 0 |
| 9 | LayerSweepCrossingMinimizer | 387 | 387 | 0 |
| 10 | LayerSweepCrossingMinimizer | 387 | 383 | 4 |
| 11 | InLayerConstraintProcessor | 383 | 382 | 1 |
| 12 | LabelAndNodeSizeProcessor | 382 | 374 | 8 |
| 13 | InnermostNodeMarginCalculator | 374 | 368 | 6 |
| 14 | CommentNodeMarginCalculator | 368 | 366 | 2 |
| 15 | BKNodePlacer | 366 | 330 | 36 |
| 16 | LayerSizeAndGraphHeightCalculator | 330 | 298 | 32 |
| 17 | OrthogonalEdgeRouter | 298 | 205 | 93 |
| 18 | EndLabelSorter | 205 | 79 | 126 |
| 19 | ReversedEdgeRestorer | 79 | 63 | 16 |
| 20 | EdgeAndLayerConstraintEdgeReverser | 63 | 28 | 35 |
| 21 | OrthogonalEdgeRouter | 28 | 9 | 19 |
| 22 | ReversedEdgeRestorer | 9 | 7 | 2 |
| 23 | NorthSouthPortPostprocessor | 7 | 4 | 3 |
| 24 | SplineEdgeRouter | 4 | 0 | 4 |

## First Failure By Step

- step 0: 1050
- step 3: 2
- step 10: 4
- step 11: 1
- step 12: 8
- step 13: 6
- step 14: 2
- step 15: 36
- step 16: 32
- step 17: 93
- step 18: 126
- step 19: 16
- step 20: 35
- step 21: 19
- step 22: 2
- step 23: 3
- step 24: 4
