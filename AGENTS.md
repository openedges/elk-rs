# ELK Layout Kernel for Pure Rust

## 프로젝트 목표
- ELK Layout Java Version을 Pure Rust 버전으로 포팅한다.
- Java Version의 기능, API, 테스트를 동일하게 이식한다.
- 데이터 모델/파일 구조를 Java와 동일하게 유지한다.
- 빌드/테스트/Clippy를 항상 통과 상태로 유지한다.
- 성능 측정 자동화를 유지하고 Java 대비 성능 회귀를 방지한다.
- 라이선스는 Java Version과 동일하게 유지한다.

## 문서 운영 원칙
- `AGENTS.md`: 작업 방식, 품질 게이트, 현재 핵심 우선순위만 관리한다.
- `HISTORY.md`: 단계별 구현/검증/수치 변화/이슈 분석 등 진행 이력을 모두 기록한다.
- 새 단계 완료 시 `HISTORY.md`를 먼저 갱신하고, 필요 시 `AGENTS.md`의 핵심 스냅샷만 동기화한다.
- AGENTS 본문 설명은 한글로 작성한다(코드/명령어/식별자/경로는 원문 영문 유지).

## 자동 진행 규칙
- 사용자가 "계속 진행" 또는 "끝까지 진행"을 요청하면, 현재 우선순위 작업을 순서대로 진행한다.
- 막힘/불확실성만 짧게 질문하고, 그 외에는 추가 질문 없이 진행한다.
- 승인/권한이 필요한 작업은 즉시 승인 요청 후 승인되면 바로 이어서 진행한다.
- 완전 구현이 어려운 항목은 최소 동작 + TODO(파일/테스트/명령 포함)를 남기고 다음 항목으로 진행한다.
- 외부 의존성/환경 제약이 있으면 합리적 가정과 안전한 기본값으로 빌드/테스트를 유지한다.
- 테스트/빌드 실패 시 우선 수정하고 재시도한 뒤 계속 진행한다.

## 품질 게이트 (단계 종료 시 필수)
1. `cargo clippy --workspace --all-targets`
2. `cargo test --workspace`
3. 필요 시 `cargo build --workspace` 및 parity/perf 재검증
4. 변경 코드 리뷰 후 커밋 (`<scope>: <summary>`)
5. 불가/예외 사항은 `HISTORY.md`에 사유와 대안을 기록

## 현재 핵심 스냅샷 (2026-02-19)
- Full model parity(2026-02-19 재실행): `matches=1135/1439`, `drift=304`, `total_diffs=5783`, `errors=0`, `timeouts=0`, `java_non_ok=9`
- tickets parity(2026-02-19 최신): `matches=101/109`, `drift=8`, `total_diffs=112`, `errors=0`, `timeouts=0`, `java_non_ok=1`
- `phase_focus_top_low_medium`(25): `matches=25`, `drift=0`, `total_diffs=0` (2026-02-19 재실행)
- p5 low/medium 3-model(`tickets/layered/288_{a,b}`, `552_hierarchySelfLoopPorts`): `matches=2`, `drift=1`, `total_diffs=26` (`552` drift 해소, `288_b` 잔여)
- 3-model subset(`algebraic/backtrack/cartracking`): `matches=1`, `drift=2`, `total_diffs=369`
- P3 seed subset(`high-degree/labels1/variants`): `matches=3`, `drift=0`, `total_diffs=0`
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태
- 현재 우선 작업: `Step M-5` high-impact drift 축소(중점 phase: `p2_layering`, `p5_edge_routing`)와 tickets 잔여 8건(`213, 302, 352, 368, 453, 502, 665, 701`) 소거

## 구현 우선순위 스냅샷 (2026-02-19)
- P1(구현 공백 제거, 완료): `IntermediateProcessorStrategy` NoOp 10개(`ConstraintsPostprocessor`, `HypernodeProcessor`, `EndNodePortLabelManagementProcessor`, `CenterLabelManagementProcessor`, `HighDegreeNodeLayerProcessor`, `AlternatingLayerUnzipper`, `SingleEdgeGraphWrapper`, `BreakingPointInserter`, `BreakingPointProcessor`, `BreakingPointRemover`)를 모두 실제 구현으로 연결 완료. 잔여 NoOp는 0개.
- P2(p5 정합, 완료): `horizontal_graph_compactor.rs`의 scanline edge-aware 제약 계산을 포팅하고 `GraphCompactionStrategy::EdgeLength`를 `NetworkSimplexCompaction` 경로로 연결했다. 관련 품질 게이트(`cargo clippy --workspace --all-targets`, `cargo test --workspace`)는 통과.
- P3(p2 고영향 모델): seed 3개(`tests/layered/high_degree_nodes/high-degree-example.elkt`, `tests/layered/compaction_oned/labels/labels1.elkt`, `tests/core/label_placement/port_labels/variants.elkt`) 수렴 완료. 다음은 `realworld/ptolemy` 상위 drift 군(`algebraic/backtrack/cartracking` 포함)으로 확장한다.
- P4(p5 저중간 diff 소거): `tickets/layered/425_selfLoopInCompoundNode.elkt` 수렴 완료. 다음은 `tickets/layered/502_collapsingCompoundNode.elkt`(compound collapse 높이/포트 y), `453_interactiveProblems.elkt`, `665_includeChildrenDoesntStop.elkt`, `701_portLabels.elkt` 순으로 잔여 8건을 축소한다.
- 정책 TODO: `java_non_ok=9`를 포함한 `1448/1448` 목표로 확장할지, 현재처럼 `1439` 비교 분모를 유지할지 정의를 고정한다.

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
- Drift 분석: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv`

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
