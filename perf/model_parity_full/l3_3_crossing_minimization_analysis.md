# L3-3 crossing minimization 결과 비교 (2026-02-16)

## 실행 및 산출물
- 분류 재확인: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv --limit 20`
  - 결과: `ordering_diff=30`, `layering_diff=138`, `other=165`
- ordering 상세 추출: `python3 scripts/analyze_layered_ordering_diff.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv --output /tmp/l3_3_ordering_diff.tsv --limit-models 200 --limit-layers 10 --max-nodes 30`
  - 고정 산출물: `perf/model_parity_full/l3_3_ordering_diff.tsv`
- 모델별 crossing 전략 매핑: `perf/model_parity_full/l3_3_model_strategy.tsv`
- 대표 모델 trace:
  - `perf/model_parity_full/l3_3_trace/portconstraints_crossmin.log`
  - `perf/model_parity_full/l3_3_trace/issue515_crossmin.log`
  - `perf/model_parity_full/l3_3_trace/classes_crossmin.log`

## 관찰 결과
1. ordering mismatch 상세는 총 51행(헤더 제외), 30개 고유 모델에서 발생
- 레이어별 mismatch 행 수: `51`
- 고유 모델 수: `30`
- 경로 분포: `realworld 16`, `tests 10`, `tickets 3`, `examples 1`

2. 레이어 내 노드 수가 작아도(2~4개) 순서 역전이 반복됨
- 노드 수 분포(행 기준): 2개=24, 3개=11, 4개=8, 5개=2, 6개=2, 10개=4
- 대표 케이스:
  - `tickets/layered/515_polylineOverNodeOutgoing.elkt`: `n4,n3 -> n3,n4`
  - `tests/layered/node_placement/bk/classes/classes_two_samesize.elkt`: `lu_b4_1,lu_b3_1,lu_b1_2 -> lu_b1_2,lu_b3_1,lu_b4_1`

3. crossing strategy 기준으로 두 집단이 분리됨
- `INTERACTIVE`: 11개 모델(51행 중 20행)
- `(default)`(LayerSweep 기본): 19개 모델(51행 중 31행)

4. trace 비교 결과
- `examples/ports/portConstraints.elkt`(default): `crossmin:` 라인 294개 출력, phase3 sweep/port distribute가 실제 수행됨
- `tickets/layered/515_polylineOverNodeOutgoing.elkt`(INTERACTIVE): `crossmin:` 라인 0개
- `tests/layered/node_placement/bk/classes/classes_two_samesize.elkt`(INTERACTIVE): `crossmin:` 라인 0개

## 근본 원인 (확정)
1. Rust `INTERACTIVE` crossing minimization phase 미구현
- Rust 현재 구현: `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/options/crossing_minimization_strategy.rs`
  - `CrossingMinimizationStrategy::Interactive`가 `NoOpPhase`로 매핑됨
- Java 원본: `external/elk/plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/options/CrossingMinimizationStrategy.java`
  - `INTERACTIVE -> new InteractiveCrossingMinimizer()`

2. Java `InteractiveCrossingMinimizer` 자체가 Rust p3order에 아직 없음
- Java 구현 파일: `external/elk/plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/InteractiveCrossingMinimizer.java`
- Rust p3order export(`p3order/mod.rs`)에 대응 모듈/타입 부재

3. 영향
- `INTERACTIVE` 옵션 모델은 Java에서는 입력 위치 기반 재정렬 + port distribution 수행
- Rust에서는 phase3이 사실상 비활성화되어 ordering drift가 직접 발생

## 해결 방법 (실행 계획)
1. `InteractiveCrossingMinimizer` 1:1 포팅
- 신규: `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/p3order/interactive_crossing_minimizer.rs`
- Java와 동일하게 구현:
  - layer/node/port id 초기화
  - long-edge/north-south dummy 위치 기반 `get_pos` 계산
  - `IN_LAYER_SUCCESSOR_CONSTRAINTS` tie-break 적용 정렬
  - `NodeRelativePortDistributor` 초기화 및 `distributePortsWhileSweeping` 수행
  - `ORIGINAL_DUMMY_NODE_POSITION` 저장

2. 전략 wiring 수정
- `CrossingMinimizationStrategy::Interactive`를 `NoOpPhase` 대신 `InteractiveCrossingMinimizer`로 연결
- `p3order/mod.rs` export 추가

3. 회귀 테스트 추가
- 옵션 팩토리 테스트: `INTERACTIVE`가 실제 minimizer 인스턴스를 반환하는지 검증
- 모델 스모크 parity:
  - `tickets/layered/515_polylineOverNodeOutgoing.elkt`
  - `tests/layered/node_placement/bk/classes/classes_two_samesize.elkt`
  - 검증 포인트: Java와 layer order 일치

4. 재검증
- 부분 재검증: `perf/model_parity_full/l3_3_model_strategy.tsv`의 INTERACTIVE 11개 우선
- 전체 재검증: full parity 재실행 후 `ordering_diff` 감소량 확인

## 참고
- `(default)` 집단(19개)에서도 ordering drift가 남아 있으므로, Interactive 포팅 후에도 LayerSweep 집단의 추가 원인 분석은 별도로 이어가야 한다.
