# Plan: Systematic 1:1 Java-Rust Code Matching for 100% ELK Parity

## Context

ELK-RS parity is at 1150/1439 (79.9%) with 289 drift models and 5635 total diffs. After 12+ sessions of incremental diff-fixing, diminishing returns have set in. The user requested a strategic shift: **stop partial diff-reduction and pursue precise 1:1 code matching** with dense unit testing and phase-level tracing.

**Constraint**: `external/elk/` is an external submodule and MUST NOT be modified. All Java tooling goes in `scripts/java/` (like the existing `ElkModelParityExportTest.java`).

### Why Incremental Fixing Stalled
- Drift cascades: one crossing-min difference propagates to node placement and edge routing
- Rust's `LabelAndNodeSizeProcessor` has a ~500-line workaround (Phase 1: `place_ports_on_side`, `ensure_clockwise_port_order`) that **does not exist in Java**
- Java's version is trivially simple: `NodeDimensionCalculation.calculateLabelAndNodeSizes(LGraphAdapters.adapt(...))`
- The cell system was ported but **regressed** when wired (1150->980) due to LGraphAdapter bugs

### Root Cause Classification (289 drift models)
| Root Cause | Models | % |
|------------|--------|---|
| Compound node width cascade (+4/+12/+24px) | ~200 | 69% |
| Crossing min ordering cascade | ~50 | 17% |
| Self-loop bendPoints | ~22 | 8% |
| N/S port splines | ~10 | 3% |
| Individual issues | ~7 | 3% |

---

## Strategy Overview: Bottom-Up Phase-by-Phase Verification

```
[Phase 1] Java Phase Trace Runner (new file in scripts/java/)
    ↓
[Phase 2] Rust Phase Trace (mirror in elk_layered.rs)
    ↓
[Phase 3] Phase Diff Tool (Python - find exact divergence point)
    ↓
[Phase 4] Function-Level Unit Tests (per-processor Rust tests)
    ↓
[Phase 5] Fix Cell System (the #1 root cause: ~200 models)
    ↓
[Phase 6] Fix Crossing Min Precision (f32/f64: ~50 models)
    ↓
[Phase 7] Remaining Fixes (self-loops, splines, individual)
```

---

## Phase 1: Java Phase Trace Runner

**Goal**: Capture Java LGraph state after each processor step, using ELK's built-in `TestExecutionState` API.

**No modification to `external/elk/`** - create a new file following the existing pattern:

### New file: `scripts/java/ElkPhaseTraceExporter.java`
Uses ELK's white-box testing API:
```java
ElkLayered elkLayered = new ElkLayered();
TestExecutionState state = elkLayered.prepareLayoutTest(lgraph);
List<ILayoutProcessor<LGraph>> processors = elkLayered.getLayoutTestConfiguration(state);

for (int i = 0; i < processors.size(); i++) {
    elkLayered.runLayoutTestStep(state);
    // Serialize LGraph state to JSON after each step
    serializeSnapshot(state, processorName, outputDir);
}
```

**Output format** (per model):
```
trace/{model}/
  step_00_import.json
  step_01_EdgeAndLayerConstraintEdgeReverser.json
  step_02_PortSideProcessor.json
  ...
  step_15_LabelAndNodeSizeProcessor.json    # THE KEY ONE
  ...
  step_final.json
```

Each snapshot captures per-node: `{id, x, y, width, height, ports: [{id, x, y, side, labels}], labels: [{x, y, w, h}]}` and per-edge: `{id, bendPoints, labels}`.

**Build**: Same as existing `ElkModelParityExportTest.java` - compile against ELK JARs from `external/elk/`.

### Modified: `scripts/run_java_phase_trace.sh`
Shell script to compile and run the trace exporter for selected models.

---

## Phase 2: Rust Phase Trace (Mirror)

**Goal**: Add identical trace recording to Rust's pipeline.

### Modified: `plugins/org.eclipse.elk.alg.layered/src/.../elk_layered.rs`
Add a trace hook in `execute_phases()` / `layout()` that serializes LGraph state after each processor runs:
```rust
fn layout_with_trace(&self, lgraph: &mut LGraph, trace_dir: Option<&Path>) {
    for (i, processor) in self.processors.iter().enumerate() {
        processor.process(lgraph);
        if let Some(dir) = trace_dir {
            serialize_snapshot(lgraph, i, processor_name, dir);
        }
    }
}
```

### New: `plugins/org.eclipse.elk.alg.layered/src/.../trace_recorder.rs`
LGraph serialization to JSON matching the Java format exactly.

### Modified: `plugins/org.eclipse.elk.graph.json/src/bin/model_parity_layout_runner.rs`
Add `--trace-dir` CLI option to enable phase tracing.

---

## Phase 3: Phase Diff Tool

### New: `scripts/compare_phase_traces.py`
Compares Java and Rust phase traces side-by-side:
```
$ python scripts/compare_phase_traces.py trace/java/verticalOrder/ trace/rust/verticalOrder/

Step 00 (import):                    MATCH (0 diffs)
Step 01 (EdgeAndLayerConstraintEdgeReverser): MATCH (0 diffs)
...
Step 15 (LabelAndNodeSizeProcessor): DRIFT (3 diffs) ← FIRST DIVERGENCE
  node "interactive": width 68 vs 44
Step 16 (BKNodePlacer):              DRIFT (3 diffs, cascaded)
```

This instantly pinpoints **the exact processor** where drift begins for each model.

---

## Phase 4: Function-Level Unit Tests

**Goal**: For each processor where drift appears, write Rust tests using Java snapshots as ground truth.

### Test Infrastructure: `tests/phase_parity/mod.rs`
```rust
/// Load a Java phase snapshot and run a single Rust processor
/// Compare output against the next Java snapshot
fn test_processor_parity(model: &str, step_before: usize, step_after: usize) {
    let input = load_java_snapshot(model, step_before);
    let expected = load_java_snapshot(model, step_after);
    let mut graph = deserialize_lgraph(&input);
    run_processor(&mut graph, step_after);
    assert_lgraph_eq(&graph, &expected);
}
```

### Priority test targets (in order):
1. **LabelAndNodeSizeProcessor** - the #1 divergence point
2. **PortListSorter** - port ordering affects everything downstream
3. **BarycenterHeuristic** - crossing min core
4. **BKNodePlacer** - coordinate assignment (already verified identical in isolation)
5. **OrthogonalEdgeRouter** - edge routing

### Test fixtures: `tests/phase_parity/fixtures/`
Java phase snapshots checked into the repo for CI reproducibility.

---

## Phase 5: Fix Cell System (Highest Impact)

This is the **highest-impact fix** affecting ~200+ models.

### 5a. Fix LGraphAdapters
The cell system regression (1150->980) was caused by LGraphAdapters providing wrong data. Key issues to fix:
- `LNodeAdapter::get_property<EnumSet<SizeConstraint>>` returns empty (property type mismatch)
- `LPortAdapter::get_position()` returns (0,0) during calculation
- Port ordering may differ from Java's `LGraphAdapters.adapt()`

**Approach**: Use phase traces to compare adapter output at each call site.

**Files**:
- `plugins/org.eclipse.elk.alg.layered/src/.../graph/transform/l_graph_adapters.rs`

### 5b. Fix process_node
Once adapters are correct, verify `process_node` produces identical output:

**Files**:
- `plugins/org.eclipse.elk.alg.common/src/.../nodespacing/node_label_and_size_calculator.rs`

### 5c. Wire Cell System & Remove Workaround
1. Make `LabelAndNodeSizeProcessor` match Java exactly:
   ```rust
   // Match Java: just delegate to NDC
   NodeDimensionCalculation::calculate_label_and_node_sizes(
       &LGraphAdapters::adapt(lgraph, true, true, |n| n.node_type() == NodeType::Normal)
   );
   // + external port dummy label handling
   ```
2. Remove Phase 1 workaround (`place_ports_on_side`, `ensure_clockwise_port_order`)

**Files**:
- `plugins/org.eclipse.elk.alg.layered/src/.../intermediate/label_and_node_size_processor.rs`
- `plugins/org.eclipse.elk.alg.common/src/.../nodespacing/node_dimension_calculation.rs`

### 5d. Validate
Run full parity. Expected: significant jump from 1150 baseline.

---

## Phase 6: Crossing Minimization Precision

**Goal**: Eliminate f32/f64 precision differences causing ~50 model drifts.

Already identified:
1. `port_ranks` / `port_barycenter` → use f32 (Java uses `float[]`)
2. `ForsterConstraintResolver` barycenter comparison → f32 cast
3. `RANDOM_AMOUNT = 0.07f` → f32 literal

**Files**:
- `plugins/org.eclipse.elk.alg.layered/src/.../p3order/abstract_barycenter_port_distributor.rs`
- `plugins/org.eclipse.elk.alg.layered/src/.../p3order/forster_constraint_resolver.rs`
- `plugins/org.eclipse.elk.alg.layered/src/.../p3order/barycenter_heuristic.rs`

Use phase traces to verify crossing min output matches Java node ordering.

---

## Phase 7: Remaining Individual Fixes

With Phases 5-6 complete, re-run parity and categorize remaining drift:
- Self-loop routing differences (~22 models)
- N/S port spline segment count (~10 models)
- HorizontalCompactor (currently NoOp)
- Individual model quirks

---

## Execution Order & Dependencies

```
Phase 1 (Java trace) ──┐
                        ├── Phase 3 (diff tool) ── Phase 4 (unit tests) ── Phase 5 (cell system fix)
Phase 2 (Rust trace) ──┘                                                        ↓
                                                                          Phase 6 (crossing min)
                                                                                 ↓
                                                                          Phase 7 (remaining)
```

Phases 1 & 2 can run in parallel. Phase 5 is the highest-impact work.

---

## Files to Create
| File | Purpose |
|------|---------|
| `scripts/java/ElkPhaseTraceExporter.java` | Java phase trace runner (uses ELK TestExecutionState API) |
| `scripts/run_java_phase_trace.sh` | Build & run Java trace exporter |
| `scripts/compare_phase_traces.py` | Phase-by-phase diff tool |
| `plugins/.../trace_recorder.rs` | Rust LGraph snapshot serialization |
| `tests/phase_parity/mod.rs` | Phase parity test infrastructure |
| `tests/phase_parity/fixtures/` | Java phase snapshots as test fixtures |

## Files to Modify
| File | Change |
|------|--------|
| `plugins/.../elk_layered.rs` | Add trace hooks in execute_phases |
| `plugins/.../model_parity_layout_runner.rs` | Add --trace-dir CLI option |
| `plugins/.../l_graph_adapters.rs` | Fix adapter methods for cell system |
| `plugins/.../label_and_node_size_processor.rs` | Remove workaround, match Java exactly |
| `plugins/.../node_dimension_calculation.rs` | Wire cell system correctly |
| `plugins/.../node_label_and_size_calculator.rs` | Fix process_node |
| `plugins/.../abstract_barycenter_port_distributor.rs` | f32 port_ranks |
| `plugins/.../forster_constraint_resolver.rs` | f32 barycenter comparison |
| `plugins/.../barycenter_heuristic.rs` | f32 RANDOM_AMOUNT |

## Verification
1. **Phase traces**: `compare_phase_traces.py` pinpoints exact divergence point per model
2. **Unit tests**: Each processor verified independently via `tests/phase_parity/`
3. **Full parity**: `model_parity_layout_runner` + `compare_model_parity_layouts.py`
4. **Regression guard**: Any change must not reduce the 1150/1439 baseline
