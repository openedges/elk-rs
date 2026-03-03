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

## 현재 핵심 스냅샷 (2026-03-03, 100% parity)
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
- **성능 기준선**: Rust **1.87x faster than Java** (`layered_xlarge` ~248ms vs 463.21ms, **0.54x** ratio, mimalloc, synthetic 25/8 baseline)
  - 이전 1,576ms → 972ms → 794ms → 702ms → 672ms → 641ms → 620ms → 626ms → 575ms → 572ms → 561ms → ~426ms → ~272ms → **~248ms** (**-84.3%** 누적), Phase 1-19 순차 최적화 반영 (상세: `HISTORY.md`)
  - **성능 최적화 실질 완료** (Phase 1-19): profile 매우 flat (개별 함수 <2%), 88% 순수 연산, 12% 인프라(malloc 7.9%, hash 4.3%)
  - 완료: Phase 1-17 (이전 상세 내역), Phase 18 mimalloc + batch lock, Phase 19 Property key Cow<'static, str> interning
  - 추가 최적화 조사 완료 (미실행): snapshot.clone() take 패턴 (회귀 확인 후 revert), FxHashMap→Vec 직접 인덱싱 (299+ 사이트 변경 필요), 전면 arena 전환 (~10-20ms 기대, 대규모 리팩토링 대비 효과 제한적)
  - 차기: 기능 개선/API 확장 등 비성능 작업으로 전환
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
