# elk-rs Custom Features

이 문서는 Java ELK v0.11.0에 없는 elk-rs 고유 확장 기능을 설명합니다.
`custom/0.11.0` 브랜치에서 관리됩니다.

---

## 1. Grid Snap (`gridSnap.gridSize`)

### 개요

노드의 크기와 위치를 지정된 grid 배수로 정렬하여 깔끔한 레이아웃을 생성합니다.

### Property

| Property ID | 타입 | 기본값 | 설명 |
|---|---|---|---|
| `org.eclipse.elk.alg.layered.gridSnap.gridSize` | `f64` | `0.0` | Grid 크기 (0 이하이면 비활성) |

### 사용법

```json
{
  "id": "root",
  "layoutOptions": {
    "algorithm": "layered",
    "org.eclipse.elk.alg.layered.gridSnap.gridSize": 10,
    "org.eclipse.elk.padding": "[left=10, top=10, right=10, bottom=10]"
  },
  "children": [
    { "id": "n1", "width": 37, "height": 53 },
    { "id": "n2", "width": 41, "height": 27 }
  ],
  "edges": [
    { "id": "e1", "sources": ["n1"], "targets": ["n2"] }
  ]
}
```

결과: 노드 크기가 40×60, 50×30 (ceil snap)으로, 위치가 10의 배수로 정렬됩니다.

### Rust API

```rust
use org_eclipse_elk_alg_layered::org::eclipse::elk::alg::layered::options::LayeredOptions;

// ElkGraph 루트 노드에 property 설정
set_property(graph, LayeredOptions::GRID_SNAP_GRID_SIZE, Some(10.0));
```

### 동작 방식

3개의 intermediate processor가 layered 알고리즘 파이프라인에 삽입됩니다:

```
BEFORE_P4: GridSnapSizeProcessor      ← 노드 크기 ceil snap
      P4:  NodePlacement              ← snapped 크기 기반 port 분배
 AFTER_P4: GridSnapPositionProcessor   ← 노드 위치 round snap
      P5:  EdgeRouting                 ← snapped 좌표 기반 edge routing
 AFTER_P5: HierarchicalNodeResizer     ← snapped 좌표 기반 parent 크기
 AFTER_P5: GridSnapGraphSizeProcessor  ← graph.size/offset snap
 AFTER_P5: DirectionPostprocessor      ← 좌표 변환 (Left/Down/Up)
```

#### GridSnapSizeProcessor (BEFORE_P4)
- **대상**: `NodeType::Normal` 노드만 (dummy 노드 제외)
- **연산**: `size = ceil(size / gridSize) * gridSize`
- **이유**: P4 (NodePlacement)가 snapped size 기반으로 port를 분배하도록 함
- **ceil 사용**: 컨텐츠(labels 등)가 항상 node 안에 들어가도록 보장

#### GridSnapPositionProcessor (AFTER_P4)
- **대상**: `NodeType::Normal` 노드만
- **연산**: `position = round(position / gridSize) * gridSize`
- **이유**: P5 (EdgeRouting)과 HierarchicalNodeResizer가 snapped 좌표를 사용하도록 함

#### GridSnapGraphSizeProcessor (AFTER_P5)
- **대상**: `graph.size`와 `graph.offset`
- **연산**: size는 `ceil`, offset은 `round`
- **이유**: `DirectionPostprocessor`의 mirror 변환 시 grid alignment 보존
  - `mirror_x: new_x = (graph.size - 2*graph.offset) - node_width - old_x`
  - 모든 항이 grid 배수이면 결과도 grid 배수

### 주의사항

1. **Padding**: 최종 ElkGraph 좌표 = `내부 위치 + graph.offset + padding`. padding이 grid 배수가 아니면 절대 좌표가 grid 배수가 되지 않습니다. 기본 padding은 12.0이므로, grid snap을 사용할 때는 padding을 grid 배수로 설정하세요.

2. **SizeConstraints**: `NODE_SIZE_CONSTRAINTS`가 비어있으면 snapped 크기가 ElkGraph에 기록되지 않습니다. `MinimumSize` 등을 설정하세요.

3. **Port/Edge 정렬**: Port 자유축과 edge bend point는 별도로 snap하지 않습니다. Port는 snapped node size 기반으로 P4에서 자연 분배되고, edge는 snapped 좌표 기반으로 P5에서 라우팅됩니다.

4. **Dummy 노드 제외**: LongEdge, ExternalPort, NorthSouthPort, Label 등의 dummy 노드는 snap 대상이 아닙니다. 이들은 알고리즘 내부 아티팩트로 최종 출력에 나타나지 않습니다.

5. **Grid 크기 제약**: `gridSize`가 `min(nodeSpacing)/2`보다 크면 노드가 겹칠 수 있습니다.

6. **Hierarchy**: 각 nested graph가 독립적인 processor pipeline을 실행하므로, compound node마다 `gridSnap.gridSize`를 설정해야 합니다.

7. **기존 parity 영향 없음**: `gridSize` 기본값이 0.0이므로 processor가 즉시 반환하여 기존 동작에 영향 없습니다.

### 수정된 파일

| 파일 | 변경 내용 |
|------|-----------|
| `options/layered_options.rs` | `GRID_SNAP_GRID_SIZE` property 정의 |
| `intermediate/grid_snap_processor.rs` | 3개 processor 구현 (신규) |
| `intermediate/mod.rs` | 모듈 등록 + re-export |
| `intermediate/intermediate_processor_strategy.rs` | enum variant + factory 추가 |
| `graph_configurator.rs` | processor 파이프라인 등록 |

모든 파일 경로는 `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/` 기준입니다.

### 테스트

28개 테스트 (13 unit + 15 integration):

**Unit tests** (`tests/intermediate/grid_snap_processor_test.rs`):
- 비활성 조건: `disabled_by_default`, `zero_grid_size_ignored`, `negative_grid_size_ignored`
- Size snap: `node_size_ceil`, `node_size_exact_no_change`
- Position snap: `node_position_round`, `position_round_midpoint`, `negative_positions`
- Graph snap: `graph_size_and_offset`
- 필터링: `skips_dummy_nodes`, `mixed_normal_and_dummy_in_layer`
- 다중 레이어: `multiple_nodes_multiple_layers`
- 비정수 grid: `non_integer_grid_size`

**Integration tests** (`tests/intermediate/grid_snap_integration_test.rs`):
- 방향 4종: `full_pipeline_right/left/down/up`
- 토폴로지: `fan_out_topology`, `diamond_topology`, `larger_graph`, `isolated_node`
- 컴포넌트: `disconnected_components`
- 크기 보장: `size_ceil_not_shrunk`, `already_aligned_unchanged`
- 경계값: `port_boundary_aligned`, `different_grid_size`
- 비활성: `no_effect_without_property`, `zero_grid_size_no_effect`

---

## 2. Ignore Edge in Layer (`ignoreEdgeInLayer`)

NetworkSimplexLayerer에서 특정 edge를 레이어 할당 시 무시하는 기능입니다.
상세 내용은 `custom/ignore-edge-in-layer` 브랜치를 참조하세요.

---

## 3. In-Layer Edge Routing

같은 레이어 내 노드 간 edge를 라우팅하는 기능입니다.
상세 내용은 `custom/in-layer-edge-routing` 브랜치를 참조하세요.
