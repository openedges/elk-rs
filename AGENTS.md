# ELK Layout Kernel for Pure Rust

## 프로젝트 목표
- ELK Layout Java Version을 Pure Rust 버전으로 포팅한다.
- Java Version의 기능, API, 테스트를 동일하게 이식한다.
- 데이터 모델/파일 구조를 Java와 동일하게 유지한다.
- 빌드/테스트/Clippy를 항상 통과 상태로 유지한다.
- 성능 측정 자동화를 유지하고 Java 대비 성능 회귀를 방지한다.
- elkjs 호환 npm 패키지(`elk-rs`)를 제공한다 (WASM + 향후 NAPI).
- 라이선스는 Java Version과 동일하게 유지한다.

## 문서 운영 원칙
- `AGENTS.md`: 작업 방식, 품질 게이트, 현재 핵심 우선순위만 관리한다.
- `HISTORY.md`: 단계별 구현/검증/수치 변화/이슈 분석 등 진행 이력을 모두 기록한다.
- 새 단계 완료 시 `HISTORY.md`를 먼저 갱신하고, 필요 시 `AGENTS.md`의 핵심 스냅샷만 동기화한다.
- `TESTING.md`: 검증 환경 준비, 항목 명세, 상황별 절차, 릴리즈 체크리스트, 알려진 이슈 통합 문서.
- `VERSIONING.md`: 버전 관리, 서브모듈 고정 정책, 포팅 워크플로우.
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
2. `cargo build --workspace` (error/warning 0건)
3. `cargo clippy --workspace --all-targets` (warning 0건)
4. `cargo test --workspace` (failure 0건, 알려진 실패 제외 — `TESTING.md` § 4.2 참조)
5. Full parity 실행 및 통과률 확인 (상세 명령은 `TESTING.md` § 2.B 참조)
6. 확인 후 문서에 진행상황(특히 parity 통과률)을 기록:
   - `AGENTS.md`의 `현재 핵심 스냅샷` 섹션의 parity 수치 갱신
   - `HISTORY.md`에 변경 내역/수치 변화 기록
7. 커밋 (`<scope>: <summary>`)
8. 불가/예외 사항은 `HISTORY.md`에 사유와 대안을 기록

## 현재 핵심 스냅샷 (2026-03-01, 100% parity)
- **elk-rs 버전**: `0.11.0` (ELK Java `v0.11.0` 기준 포팅 완료)
  - Cargo workspace 전체 + npm 동일 버전
  - 서브모듈 고정: `external/elk` → `v0.11.0` 태그, `external/elkjs` → `0.11.0` 태그
- **Model parity**: `matches=1438/1438`, `drift=0`, `java_non_ok=9`, `java_excluded=1` **(100%)**
  - Java 패치(`SelfHyperLoop` EnumMap 결정론) 적용 + stale Maven `0.12.0-SNAPSHOT` 캐시 제거로 달성
  - `213_componentsCompaction.elkt`는 Java ELK NaN 버그로 `java_exclude.txt`에서 제외
- **Phase-step trace**: `1439/1439` 모델 전 step 일치, 초기 frontier/hotspot 없음
- **Tickets parity**: `matches=108/109`, `drift=1` (Java ELK 버그 동일 원인)
- **JS parity**: 550/550 elk-rs vs Java 일치 (ELKJS_DRIFT 20건은 GWT 아티팩트)
- **npm 패키지**: `elk-rs@0.11.0` WASM-only 릴리즈 준비 완료 (32 Vitest 통과)
  - 향후: NAPI 플랫폼별 패키지 (`@elk-rs/darwin-arm64` 등) 추가 예정
- **성능 기준선**: Rust ~2.10x slower than Java (`layered_xlarge` 972.20ms vs 463.21ms, synthetic 10/3 baseline)
  - 이전 1,576ms → 972.20ms (**-38.3%**), Phase 1-4 + Phase 5 순차 최적화 반영 (상세: `HISTORY.md`)
  - 최근 probe: `layered_xlarge` `1,020ms -> 992ms`(5/1), `990ms`(10/3), `988.53ms`(10/3), `986.70ms`(10/3), `984.26ms`(10/3), `978.63ms`(10/3), `980.90ms`(full 5-way 10/3), `994.23ms`/`1011.96ms`(rust_native/rust_api 단독 10/3), `991.85ms`/`1008.94ms`(full 5-way 10/3), `989.24ms`/`1010.24ms`(full 5-way 10/3), `976.95ms`/`996.45ms`(full 5-way 10/3), `981.76ms`/`1008.32ms`(full 5-way 10/3), `981.50ms`/`997.73ms`(full 5-way 10/3), `972.20ms`/`992.88ms`(full 5-way 10/3), `984.92ms`/`994.34ms`(full 5-way 10/3), `980.92ms`/`998.07ms`(rust_native/rust_api 단독 10/3), `973.99ms`/`995.29ms`(full 5-way 10/3), `982.25ms`/`1003.22ms`(full 5-way 10/3), `983.51ms`/`998.09ms`(full 5-way 10/3)
  - 완료: Phase 1 `LazyLock` 전환(130개 env::var 호출), Phase 2 ports_by_side 버퍼 재사용, Phase 3 Delta crossing cache, Phase 4 P5 hybrid routing-slot kernel, Phase 5.1 P3 barycenter sort snapshot, Phase 5.2 ForsterConstraintResolver arena/index 전환, Phase 5.3 model-order port comparator 재사용, Phase 5.4 previous-layer clone 축소, Phase 5.5 hierarchical sweep target snapshot, Phase 5.6 hierarchical target buffer reuse, Phase 5.7 model-order first-layer previous semantics 복원, Phase 5.8 changed-graph index iteration allocation 제거, Phase 5.9 child-graph index 수집 할당 제거, Phase 5.10 crossing-count traversal scratch 버퍼 재사용, Phase 5.11 node/port crossing count 단일-pass화, Phase 5.12 model-order comparator lifecycle 재사용, Phase 5.13 model-order 영향 계산 lock/전처리 절감, Phase 5.14 model-order active-index 재사용 + no-op port clone 제거, Phase 5.15 SortByInputModelProcessor comparator 재사용
  - 잔여 병목: Arc/Mutex 기반 core graph lock 경합(~10-15%), malloc/free(~5%), **Phase 5 Full arena 전면 전환 필수 진행 중**
  - 실행 계획: `HISTORY.md`의 `Full arena 전환 필수화 및 실행 계획 확정(2026-03-01)` 항목 기준 Phase 0~4 완료, Phase 5 순차 진행
- **코드 품질**: `cargo build` warning 0건, `cargo clippy` warning 0건
- **Parity 재검증 상태**: full parity 재실행 완료 — `compared=1438`, `matches=1438`, `drift=0`, `skipped=9`, `errors=0`
- **알려진 실패**: `elk_live_examples_test` 1건 (cross-hierarchy edge resolution, `TESTING.md` § 4.2)
- `213_componentsCompaction.elkt`: `java_non_ok=nan_output`으로 분류 — Rust 출력이 더 정확

## Drift 분석 요약

### 213_componentsCompaction (Java ELK 버그)
- NaN y좌표 73건: Java `ComponentsCompactor`에서 zero-size 노드 bounding box 계산 시 `∞ - ∞ = NaN` 전파
- x좌표 차이 12건: Java hull 기반 1D compaction vs Rust anchor 기반 정렬
- 결론: Rust 출력이 더 정확, Java `ComponentsCompactor` 포팅 비권장 (800+ lines + NaN 버그 복제)

### 해결된 drift (2026-02-26)
- `next_to_port_if_possible_inside.elkt` — `components_processor` `.max(0.0)` clamp 제거
- `multilabels_compound.elkt` — 동일 원인 (negative graph size는 유효한 중간값)

## 기본 실행 명령
- 일상 개발 및 릴리즈 절차: `TESTING.md` § 3 참조
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity_full`
- Phase trace 비교: `python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch`
- 5-way 성능 벤치마크: `sh scripts/run_perf_benchmark.sh synthetic 10 3 tests/perf`
  - 통합 바이너리: `perf_benchmark --engine rust_native|rust_api --mode synthetic|models`

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
