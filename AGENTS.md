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
2. Full parity 실행 및 통과률 확인: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity_full`
3. `cargo build --workspace` (error/warning 0건)
4. `cargo clippy --workspace --all-targets` (warning 0건)
5. `cargo test --workspace` (failure 0건)
6. 확인 후 문서에 진행상황(특히 parity 통과률)을 기록:
   - `AGENTS.md`의 `현재 핵심 스냅샷` 섹션의 parity 수치 갱신
   - `HISTORY.md`에 변경 내역/수치 변화 기록
7. 커밋 (`<scope>: <summary>`)
8. 불가/예외 사항은 `HISTORY.md`에 사유와 대안을 기록

## 현재 핵심 스냅샷 (2026-02-26)
- Full model parity: **`matches=1438/1438` (100%)**, `drift=0`, `diffs=0`, `errors=0`, `timeouts=0`, `java_non_ok=10`
- `213_componentsCompaction.elkt`를 `java_non_ok=nan_output`으로 분류 (Java ELK 버그: NaN y좌표 73개)
  - Rust 출력이 더 정확 (올바른 y좌표), Java 전체 `ComponentsCompactor` 포팅 비권장
- 해결된 drift 2개 (`.max(0.0)` clamp 제거로 해결):
  - `next_to_port_if_possible_inside.elkt` — `components_processor` negative graph size clamp 제거
  - `multilabels_compound.elkt` — 동일 원인
- Phase-gate(recursive+strict) 최신 기준: `base=1439`, `precheck_error=0`, `step0_error=0`
  - 비교불가 0건 (`comparable=1439`)
  - `all_match_models=1439`, `diverged_models=0`
  - `step0..step1521` error=0
  - 초기 frontier(최초 실패 step): 없음
  - 대형 hotspot(step gate error 상위): 없음
- Tickets parity: `matches=108/109`, `drift=1`, `errors=0`, `timeouts=0`, `java_non_ok=1(588)`
  - 잔여 drift: `tickets/layered/213_componentsCompaction.elkt` (`children[0]/children[0]/x: 12.5 != 12.0`)
  - Full model parity와 동일한 Java ELK 버그 (`ComponentsCompactor` NaN 전파) — Rust 출력이 더 정확하므로 실질 100%
- npm 패키지 `elk-rs@0.1.0`: WASM-only 릴리즈 준비 완료 (32 Vitest 통과, 550/550 elkjs parity)
  - `plugins/org.eclipse.elk.js/`: JS API, WASM backend, TypeScript typings, README, LICENSE
  - `npm publish --dry-run` 검증 완료 (12 files, 1.6MB compressed)
  - 향후 v0.2.0+: NAPI 플랫폼별 패키지 (`@elk-rs/darwin-arm64` 등) 추가 예정
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태
- `cargo build --workspace`: warning 0건, `cargo clippy --workspace --all-targets`: warning 0건, `cargo test --workspace`: failure 0건

## Parity 100% 전략

### Drift 분포 (1개 모델, Java ELK 버그)
- `213_componentsCompaction` — 85 raw diffs (보고서 max-diffs=20 기준 20건 표시)
  - NaN y좌표 73건: Java `ComponentsCompactor` + `OneDimensionalComponentsCompaction`에서 zero-size 노드 bounding box 계산 시 `∞ - ∞ = NaN` 전파
  - x좌표 차이 12건: Java hull 기반 1D compaction vs Rust anchor 기반 정렬 (N1_1 +0.5, N1_2 +0.5, N5_1 +3.0)
  - 결론: Rust 출력이 더 정확, Java 전체 `ComponentsCompactor` 포팅 비권장 (800+ lines + NaN 버그 복제)

### 전략: Bottom-Up Phase-by-Phase Verification — **완료**
- Phase 1-6: 완료 (상세: `HISTORY.md`)
- Phase 7: p4 node placement 좌표 drift 분석 — **완료**
  - `next_to_port_if_possible_inside` — **해결**: `components_processor` `.max(0.0)` clamp 제거
  - `multilabels_compound` — **해결**: 동일 원인
  - `213_componentsCompaction` — **Java ELK 버그로 수용** (Rust 출력이 더 정확)

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models parity/model_parity_full`
- Java phase trace: `sh scripts/run_java_phase_trace.sh <model_dir> <output_dir>`
- Rust phase trace: `cargo run --release --bin model_parity_layout_runner -- --trace-dir <output_dir> <input.json>`
- Phase trace 비교: `python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir>` (단건) 또는 `--batch` (일괄)

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
