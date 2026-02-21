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

## 현재 핵심 스냅샷 (2026-02-21)
- Full model parity: `matches=1164/1439` (80.9%), `drift=275`, `diffs=5374`, `errors=0`, `timeouts=0`, `java_non_ok=9`
  - 이전: 1150 → self-loop fix + crossing min early return + port label fix → **1164**
- 남은 drift 275개는 모두 crossing min random state divergence에서 캐스케이딩
- 포팅/테스트/빌드/성능 자동화 파이프라인은 운영 상태
- `cargo build --workspace`: warning 0건, `cargo clippy --workspace --all-targets`: warning 0건

## Parity 100% 전략

### Drift 분포 (275개 모델)
- diff 분류: coordinate(80.1%), section(15.4%), structure(3.9%), ordering(0.3%), label(0.3%)
- 275개 모두 crossing min random state divergence에서 캐스케이딩
- crossing min 구조적 컴포넌트는 전수 검증 완료 (Java와 동일)

### 전략: Bottom-Up Phase-by-Phase Verification
- 상세 계획: `perf/PARITY_PLAN.md`
- Phase 1-2: Java/Rust phase trace 도구 구축
- Phase 3: Phase diff tool (프로세서별 divergence point 탐지)
- Phase 4: 프로세서별 단위 테스트
- Phase 5: Cell System 수정 (최대 영향, ~200 모델) — LGraphAdapters 버그 수정 필요
- Phase 6: Crossing min f32/f64 정밀도 차이 해소 (~50 모델)
- Phase 7: Self-loop, N/S port spline 등 잔여 수정

## 기본 실행 명령
- 전체 정적 점검: `cargo clippy --workspace --all-targets`
- 전체 테스트: `cargo test --workspace`
- Full parity: `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
- Drift 분석: `python3 scripts/analyze_layered_drift.py --diff-details perf/model_parity_full/diff_details.tsv --manifest perf/model_parity_full/rust_manifest.tsv`

## 진행 기록 위치
- 상세 진행 이력/완료 단계/드리프트 분석/실험 로그/TODO: `HISTORY.md`
