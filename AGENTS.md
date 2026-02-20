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
1. 코드 변경 후 코드리뷰 실행
2. Full parity 실행 및 통과률 확인: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
3. `cargo build --workspace` (error/warning 0건)
4. `cargo clippy --workspace --all-targets` (warning 0건)
5. `cargo test --workspace` (failure 0건)
6. 확인 후 문서에 진행상황(특히 parity 통과률)을 기록:
   - `AGENTS.md`의 `현재 핵심 스냅샷` 섹션의 parity 수치 갱신
   - `HISTORY.md`에 변경 내역/수치 변화 기록
7. 커밋 (`<scope>: <summary>`)
8. 불가/예외 사항은 `HISTORY.md`에 사유와 대안을 기록

## 현재 핵심 스냅샷 (2026-02-20)
- Full model parity: `matches=1150/1439` (79.9%), `drift=289`, `errors=0`, `timeouts=0`, `java_non_ok=9`
  - 이전: 1116(HEAD cb94b09) → routing_director.rs regression revert + INSIDE port label offset fix → **1150**
- tickets parity: `matches=107/109`, `drift=2`, `errors=0`, `timeouts=0`, `java_non_ok=1`
- tickets 잔여 drift: `tickets/layered/213_componentsCompaction.elkt`, `tickets/layered/701_portLabels.elkt`
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태

## Parity 100% 전략 (Phase 1→2→3)

### Drift 분포
- 289개 drifted 모델 중 277개(95.8%)가 20-diff cap 도달
- hierarchical ptolemy: 182개(63%), flattened ptolemy: 88개(30%), tests/examples: 17개(6%), tickets: 2개(1%)
- first diff의 92.1%가 노드 좌표(x/y) — 체계적 원인

### 근본 원인
- `alg.common/nodespacing` cell system 불완전: Java 25개 파일(~5,368 LOC) 중 핵심 4개 calculator 누락
  - `HorizontalPortPlacementSizeCalculator` (391 LOC) — N/S 포트 최소 너비
  - `PortPlacementCalculator` (404 LOC) — 포트 최종 위치
  - `PortLabelPlacementCalculator` (526 LOC) — 포트 라벨 위치
  - `CellSystemConfigurator` (163 LOC) — 셀 크기 기여 플래그

### Phase 1: Cell System 충실한 포팅 (최우선)
- Java의 cell system (NodeContext, PortContext, GridContainerCell, 7개 algorithm phase)을 Rust로 포팅
- 예상 ~2,500-3,000 LOC 신규/교체, ~270개 모델 해결 예상
- 상세 계획: `.claude/plans/cuddly-forging-liskov.md`

### Phase 2: 진단 도구 구축
- Processor pipeline 설정 비교, phase-level state snapshot, divergence binary search
- Phase 3의 효율을 위해 필요

### Phase 3: 잔여 Drift 해소
- ElkGraphLayoutTransferrer 보상 로직 정리
- HierarchicalPortPositionProcessor 좌표 변환 정렬
- HashSet → IndexSet 변환, self-loop routing 미세 차이
- low-hanging fruit: verticalOrder(3 diffs), next_to_port_if_possible_inside(5 diffs)

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
- Drift 분석: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv`

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
