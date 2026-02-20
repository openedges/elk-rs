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

## 단계별 상시 체크 (매 작업 단계)
1. 코드/문서 변경 직후 `cargo build --workspace`를 실행한다.
2. `cargo build` 출력의 error/warning을 0건으로 맞춘 뒤 다음 단계로 진행한다.
3. warning이 새로 발생하면 임시 무시 없이 해당 단계에서 즉시 수정한다.

## 품질 게이트 (단계 종료 시 필수)
1. `cargo clippy --workspace --all-targets`
2. `cargo test --workspace`
3. 필요 시 `cargo build --workspace` 및 parity/perf 재검증
4. 변경 코드 리뷰 후 커밋 (`<scope>: <summary>`)
5. 불가/예외 사항은 `HISTORY.md`에 사유와 대안을 기록

## 현재 핵심 스냅샷 (2026-02-20)
- Full model parity(2026-02-20 최신 report): `matches=1142/1439`, `drift=297`, `total_diffs=5710`, `errors=0`, `timeouts=0`, `java_non_ok=9`
- tickets parity(2026-02-20 최신): `matches=107/109`, `drift=2`, `total_diffs=40`, `errors=0`, `timeouts=0`, `java_non_ok=1`
- tickets 잔여 drift: `tickets/layered/213_componentsCompaction.elkt`, `tickets/layered/701_portLabels.elkt`
- tickets full-diff(`--max-diffs-per-model 5000`) 기준 총 diff는 `264`이며, `701_portLabels` 단건 diff는 `139 -> 129`로 감소
- `phase_focus_top_low_medium`(25): `matches=25`, `drift=0`, `total_diffs=0` (2026-02-19 재실행 유지)
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태
- 현재 우선 작업: `Step M-5` high-impact drift 축소(중점 phase: `p2_layering`, `p5_edge_routing`)와 tickets 잔여 2건(`213`, `701`) 소거

## 구현 우선순위 스냅샷 (2026-02-20)
- P1(구현 공백 제거, 완료): `IntermediateProcessorStrategy` NoOp 10개(`ConstraintsPostprocessor`, `HypernodeProcessor`, `EndNodePortLabelManagementProcessor`, `CenterLabelManagementProcessor`, `HighDegreeNodeLayerProcessor`, `AlternatingLayerUnzipper`, `SingleEdgeGraphWrapper`, `BreakingPointInserter`, `BreakingPointProcessor`, `BreakingPointRemover`)를 모두 실제 구현으로 연결 완료. 잔여 NoOp는 0개.
- P2(p5 정합, 완료): `horizontal_graph_compactor.rs`의 scanline edge-aware 제약 계산을 포팅하고 `GraphCompactionStrategy::EdgeLength`를 `NetworkSimplexCompaction` 경로로 연결했다. 관련 품질 게이트(`cargo clippy --workspace --all-targets`, `cargo test --workspace`)는 통과.
- P3(full parity 고영향 모델): `p2_layering`, `p5_edge_routing` 중심으로 `realworld/ptolemy` 상위 drift 군(`algebraic/backtrack/cartracking` 포함)과 `phase_focus_top_low_medium` 큐를 우선 소거한다.
- P4(tickets 잔여 2건 소거): `tickets/layered/701_portLabels.elkt`(inside port-label/side별 cell 폭 정합) 먼저 수렴시키고, 이어 `tickets/layered/213_componentsCompaction.elkt`(component compaction 좌표 정합)까지 마무리한다.
- P5(재검증/동기화): tickets `109/109` 수렴 시 full parity/phase 분석 산출물을 재생성하고 `HISTORY.md`/`AGENTS.md`를 즉시 동기화한다.
- 정책 TODO: `java_non_ok=9`를 포함한 `1448/1448` 목표로 확장할지, 현재처럼 `1439` 비교 분모를 유지할지 정의를 고정한다.

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
- Drift 분석: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv`

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
