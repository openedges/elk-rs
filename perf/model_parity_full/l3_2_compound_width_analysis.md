# L3-2 Compound Width 정합 분석 (2026-02-16)

## 요약
- 현 full parity(`perf/model_parity_full`) drift 333건 중 width/height 경로가 포함된 모델은 155건, nested width/height 경로 포함 모델은 65건.
- nested width/height drift 최상위는 `tickets/layered/701_portLabels.elkt`이며, 단건 정밀 비교 시 총 diff 259개, 이 중 width/height 68개.
- Java 대비 Rust에서 compound 하위 노드 폭이 축소되며(예: `MyNode2`: Java `270x63`, Rust `60x44`), 이 축소가 상위 compound 노드 폭/좌표 drift로 전파됨.

## 근본 원인
1. `org.eclipse.elk.alg.common`의 NodeDimensionCalculation 경로 정합 부족
- Java는 `NodeLabelAndSizeCalculator` 내부의 `PortContextCreator`, `Horizontal/VerticalPortPlacementSizeCalculator`, `PortPlacementCalculator`, `PortLabelPlacementCalculator`, `NodeSizeCalculator`를 통해 포트/포트라벨(특히 fixed `[]`)까지 포함해 노드 최소 폭/높이를 계산함.
- Rust `plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/node_label_and_size_calculator.rs`는 위 Java phase(포트 컨텍스트/포트 라벨 margin/port alignment 기반 폭 계산)가 사실상 부재하여, 포트 라벨 영향이 불완전하게 반영됨.

2. layered Phase1 보정 로직의 한계
- `label_and_node_size_processor.rs`의 Phase1은 포트 재배치 중심으로 동작하며, Java nodespacing의 포트 라벨 기반 폭 계산 전체를 대체하지 못함.
- `ELK_TRACE_NODE_SIZE=1` 추적으로 `701_portLabels`의 다수 노드가 Phase1 진입 시점에 이미 `60/80/95/110` 폭으로 시작하는 것을 확인(동일 노드 Java 결과는 `126/168/270` 등).

3. 결론
- 문제 핵심은 layered 단일 함수 보정이 아니라, `alg.common nodespacing` 계층의 Java 포트/라벨 계산 phase 누락이다.
- 따라서 L3-2 정합은 `alg.common` 포팅 작업이 선행되어야 재현 가능하다.

## 즉시 적용 가능한 해결 방법
1. Java nodespacing phase 포팅
- 대상(Java):
  - `external/elk/plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/internal/PortContext.java`
  - `.../internal/algorithm/HorizontalPortPlacementSizeCalculator.java`
  - `.../internal/algorithm/VerticalPortPlacementSizeCalculator.java`
  - `.../internal/algorithm/PortPlacementCalculator.java`
  - `.../internal/algorithm/PortLabelPlacementCalculator.java`
  - `.../internal/algorithm/NodeSizeCalculator.java`
- 대상(Rust):
  - `plugins/org.eclipse.elk.alg.common/src/org/eclipse/elk/alg/common/nodespacing/node_label_and_size_calculator.rs`
  - 필요 시 `internal/*` 모듈 분리 생성

2. layered 보정 경량화
- 위 포팅 후 `LabelAndNodeSizeProcessor` Phase1의 폭 보정 책임을 축소(포트 재배치 보조만 유지)하여 Java와 같은 책임 분리를 회복.

3. 회귀 검증
- 모델 parity 단건: `tickets/layered/701_portLabels.elkt`
- 모델 parity subset: nested width/height top N(65건에서 우선 10~20건)
- 기존 회귀: `issue_701_test`, `port_label_placement_variants_test`, `inside_port_label_test`

## TODO
- [ ] `alg.common` nodespacing 포트/라벨 phase(Java) 1:1 포팅
- [ ] `701_portLabels` 단건 parity를 `diff_count=259 -> <100` 1차 축소
- [ ] full parity 재실행 후 width/height drift 모델 재집계
