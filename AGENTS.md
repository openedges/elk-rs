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

## 현재 핵심 스냅샷 (2026-02-22)
- Full model parity: `matches=1174/1439` (81.6%), `drift=265`, `diffs=5222`, `errors=0`, `timeouts=0`, `java_non_ok=9`
  - 이전 스냅샷(2026-02-21): `1164/1439` → 최신 재검증에서 **1174/1439**로 갱신
- 남은 drift 265개는 기존 phase 분석 기준 crossing min random state divergence에서의 cascading 영향이 중심
- Phase-gate(recursive+strict) 최신 기준: `base=1439`, `precheck_error=0`, `step0_error=0`
  - 초기 frontier: `step8=41`, `step9=71`
  - 대형 hotspot: `step10=316`, `step11=615`, `step12=238`
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태
- `cargo build --workspace`: warning 0건, `cargo clippy --workspace --all-targets`: warning 0건

## Parity 100% 전략

### Drift 분포 (265개 모델)
- diff 분류: coordinate(80.0%), section(15.6%), structure(3.7%), ordering(0.3%), label(0.2%), other(0.1%)
- 265개의 drift는 기존 분석 기준 crossing min random state divergence cascading이 중심
- crossing min 구조적 컴포넌트는 전수 검증 완료 (Java와 동일)

### 전략: Bottom-Up Phase-by-Phase Verification
- Phase 1: Java phase trace 도구 — **완료**
- Phase 2: Rust phase trace 도구 — **완료**
- Phase 3: Phase diff 비교 도구 — **완료**
- Phase 4: trace 실행 및 divergence 분석 — **운영 중**
  - 최신 기준은 recursive+strict gate 산출물(`perf/model_parity/phase_gate_latest.md`)을 단일 기준으로 사용
  - 최신 step range(루트 trace 기준): `0..37`
  - 우선순위는 `first_failure_by_step` 오름차순(가장 작은 step error부터 0화)
- Phase 5: 식별된 10개 모델의 프로세서별 버그 수정 — **완료** (추가 수정 가능한 항목 없음)
  - 10개 중 실제 drift는 4개만: LifeCGAVR(PortSideProcessor), Life(PortSideProcessor), next_to_port(LayerSize), labels(BKNodePlacer)
  - labels.elkt: Java HashMap keySet() 비결정성 (PortSide identity hashCode) — **수정 불가**
  - `<=` 비교연산자 변경 시도 → 1164→1103 퇴행(-61), 롤백. `opposing_tie_break_rank` 유지가 최적
  - next_to_port_if_possible_inside: 내부 레이아웃 그래프 크기 차이 (step_000부터 diverge), stale Java ref 가능 — **수정 불요**
  - Life/LifeCGAVR: PortSideProcessor 기능적 동일 확인, 자식 sub-graph crossing-min cascade — **수정 불요**
- Phase 6: Crossing min random state divergence 해소 (~257 모델)
- Phase 7: Self-loop, N/S port spline 등 잔여 수정

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
- Drift 분석: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv`
- Java phase trace: `sh scripts/run_java_phase_trace.sh <model_dir> <output_dir>`
- Rust phase trace: `cargo run --release --bin model_parity_layout_runner -- --trace-dir <output_dir> <input.json>`
- Phase trace 비교: `python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir>` (단건) 또는 `--batch` (일괄)

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
