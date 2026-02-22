# 검증환경 종합 정리

ELK Java -> Rust 포팅의 검증은 아래 다층 게이트로 운영한다.

## 0) 테스트 플로우 (실행 우선순위)

로컬/CI 공통으로 아래 순서로 실행한다.

1. `LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets`
4. `cargo test --workspace`
5. (릴리즈/회귀 분석 단계) `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`

각 단계의 판정 기준:

- 1단계: `perf/layered_phase_wiring_parity.md`가 `status: ok`여야 함
- 2단계: build error/warning 0건
- 3단계: clippy warning 0건
- 4단계: test failure 0건
- 5단계: parity drift 수치/분포를 기록하고 `HISTORY.md` 갱신

실패 시 공통 분석 루프:

1. 실패 단계 단건 재현
2. 관련 crate/test로 범위 축소
3. 필요 시 phase trace 비교(Java/Rust)로 divergence 지점 식별
4. 원인/가설/재현 명령을 `HISTORY.md`에 기록

## 1) 코드 품질 게이트

- 목적: 빌드/테스트/정적분석의 기본 건전성 확보
- 명령:
  - `cargo build --workspace`
  - `cargo clippy --workspace --all-targets`
  - `cargo test --workspace`
- 실패 기준: error/warning/failure 1건 이상

## 2) 정적 parity (구성/메타데이터)

- 목적: Java와 Rust의 옵션/알고리즘 등록/메타데이터 불일치 탐지
- 주요 스크립트:
  - `sh scripts/check_core_options_parity.sh`
  - `sh scripts/check_core_option_dependency_parity.sh`
  - `sh scripts/check_algorithm_*_parity.sh`
- 산출물: `perf/*parity.md`

## 3) phase wiring parity (정적 구조)

- 목적: layered `GraphConfigurator`의 phase wiring(before/after/phase/processor/guard) 동등성 검증
- 명령:
  - `sh scripts/check_layered_phase_wiring_parity.sh`
  - strict: `LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh`
- 입력:
  - Java: `external/elk/plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/GraphConfigurator.java`
  - Rust: `plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/graph_configurator.rs`
- 산출물:
  - 보고서: `perf/layered_phase_wiring_parity.md`
  - 상세 TSV: `perf/layered_phase_wiring/*.tsv`

## 4) 테스트 parity (구조 레벨)

- 목적: Java 테스트 모듈/이슈 테스트의 이식 커버리지 확인
- 주요 스크립트:
  - `sh scripts/check_layered_issue_test_parity.sh`
  - `sh scripts/check_java_test_module_parity.sh`
- 주의: 메서드 semantics를 Java 테스트 엔진으로 1:1 실행하는 검증은 아님(구조/카운트/매핑 중심)

## 5) 동작 parity (실행 결과)

- 목적: 동일 모델 입력에 대해 Java layout 결과와 Rust 결과를 직접 비교
- 파이프라인:
  - `MODEL_PARITY_SKIP_JAVA_EXPORT=true sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models perf/model_parity_full`
  - 내부 단계: Java export -> Rust replay -> JSON diff
- 산출물:
  - `perf/model_parity_full/report.md`
  - `perf/model_parity_full/diff_details.tsv`
  - `perf/model_parity_full/rust_manifest.tsv`

## 6) phase/알고리즘 단위 원인 분석

- 목적: drift의 최초 divergence phase/processor를 식별하고 원인 축소
- 도구:
  - Java trace: `sh scripts/run_java_phase_trace.sh <model_dir> <output_dir>`
  - Rust trace: `cargo run --release --bin model_parity_layout_runner -- --trace-dir <output_dir> <input.json>`
  - 비교: `python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir>`
  - 배치 분석: `python3 scripts/analyze_layered_drift.py --diff-details ... --manifest ...`

## 7) 성능/회귀 게이트

- 목적: Rust baseline 대비 성능 회귀 방지 + Java 대비 편차 모니터링
- 주요 명령:
  - `PERF_COMPARE_MODE=baseline sh scripts/check_perf_regression.sh 5 3`
  - `sh scripts/check_recursive_perf_runtime_budget.sh perf/results_recursive_layout_scenarios.csv default perf/recursive_runtime_budget.md`
  - `sh scripts/check_java_perf_parity.sh ...`
  - `sh scripts/check_java_perf_parity_scenarios.sh ...`

## 8) CI 반영 상태

- 빠른 게이트: `.github/workflows/ci.yml` (`run_fast_checks.sh`)
- 전체 성능/패리티 게이트: `.github/workflows/perf.yml`
  - algorithm/core parity
  - `check_layered_phase_wiring_parity.sh`
  - 보고서/TSV 아티팩트 업로드

## 9) 운영 원칙

- 릴리즈 전에는 `RELEASE_CHECKLIST.md` 순서대로 실행한다.
- parity 수치/실험 로그/예외 사유는 `HISTORY.md`에 누적 기록한다.
- 핵심 스냅샷(요약 수치)은 `AGENTS.md`에 유지한다.
- `perf/README.md`의 `Directory policy (keep vs temporary)`를 따라 `KEEP`/`TEMP` 산출물을 분리 관리한다.

## 10) 실행 확인 순서

- 권장 순서:
  1. `LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh`
  2. `cargo build --workspace`
  3. `cargo clippy --workspace --all-targets`
  4. `cargo test --workspace`
- 2026-02-22 실행 결과:
  - 1~3 단계는 통과
  - 4 단계는 `plugins/org.eclipse.elk.alg.layered/tests/ptides_self_loop_margin_test.rs`의 `opposing_east_west_self_loop_fixedpos_extends_west_margin` 1건 실패
  - 재현 명령: `CARGO_TARGET_DIR=/tmp/elk-rs-test-target cargo test -p org-eclipse-elk-alg-layered --test ptides_self_loop_margin_test`
  - 원인 요약: 구현 로직보다는 테스트 기대값(`18`)과 실제 슬롯 배정 결과(`28`) 불일치 가능성이 높음(수정은 보류)
- 2026-02-22(phase-gate 재실행 후) 확인 결과:
  - `CARGO_TARGET_DIR=/tmp/elk-rs-phase-gate-target cargo clippy --workspace --all-targets` 통과
  - `CARGO_TARGET_DIR=/tmp/elk-rs-phase-gate-target cargo test --workspace`는 `plugins/org.eclipse.elk.alg.layered/tests/issue_425_test.rs`의 `issue_425_self_loop_in_compound_matches_java_reference_sizes` 1건 실패
  - 단건 재현: `CARGO_TARGET_DIR=/tmp/elk-rs-phase-gate-target cargo test -p org-eclipse-elk-alg-layered --test issue_425_test`
  - 실패 관측값: `Node_1 width expected 30.0, got 20.0` (원인 분석/수정 필요)

## 11) Phase-Gate 우선 검증 계획 (2026-02-22)

목표를 모델별 최종 layout match가 아니라, **phase 순서별 게이트 통과**로 재정의한다.

### 11.1 판정 규칙

1. 기준 집합은 `perf/model_parity/java/java_manifest.tsv`에서 `java_status=ok`인 전체 모델이다.
2. Java/Rust phase trace 중 한쪽이라도 없으면 `비교불가(error)`로 분류한다.
3. 비교 가능한 모델은 `compare_phase_traces.py --batch` 결과에서 `최초 non-match step`을 해당 모델의 실패 phase로 기록한다.
4. `compare_phase_traces.py`는 노드/포트/레이어를 위치 기반(positional)으로 비교하므로, 순서가 바뀌면 drift(error)로 판정한다.
5. 어떤 모델이 step `k`에서 실패하면 `k+1` 이후 step은 미판정으로 취급한다.
6. 최종 합격 조건은 `비교불가(error)=0`이고, 모든 phase step에서 `error=0`이다.

### 11.2 실행 절차 (고정)

1. Wiring parity strict
   - `LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh perf/layered_phase_wiring_parity_strict_latest.md`
2. Java phase trace 생성 (ELKT 직접 파싱 대신 JSON 입력 사용)
   - `JAVA_TRACE_EXTERNAL_ISOLATE=false JAVA_TRACE_BUILD_PLUGINS=false sh scripts/run_java_phase_trace.sh perf/model_parity/java/input /tmp/phase_gate/java_trace`
3. Rust phase trace 생성
   - `cargo run --release -p org-eclipse-elk-graph-json --bin model_parity_layout_runner -- --input-manifest perf/model_parity/java/java_manifest.tsv --output-manifest /tmp/phase_gate/rust_manifest.tsv --rust-layout-dir /tmp/phase_gate/rust_layout --pretty-print false --stop-on-error false --trace-dir /tmp/phase_gate/rust_trace`
   - 참고: timeout 모델이 trace 실행 중간에 나오면 timeout worker가 남아 후속 trace가 누락될 수 있다. known timeout 모델(`ptides_powerplant_PowerPlant.elkg/.elkt`)은 manifest 끝으로 재배치한 뒤 실행한다.
4. Batch phase 비교
   - `python3 scripts/compare_phase_traces.py /tmp/phase_gate/java_trace /tmp/phase_gate/rust_trace --batch --json > /tmp/phase_gate/phase_compare_full.json`
   - 순서 기준: `compare_phase_traces.py`는 component-run 재정렬 없이 **strict step order**로 비교한다.
5. Gate 집계
   - `python3 scripts/summarize_phase_gate.py --java-manifest perf/model_parity/java/java_manifest.tsv --rust-manifest /tmp/phase_gate/rust_manifest.tsv --java-trace-dir /tmp/phase_gate/java_trace --rust-trace-dir /tmp/phase_gate/rust_trace --compare-json /tmp/phase_gate/phase_compare_full.json --output-json perf/model_parity/phase_gate_latest.json --output-md perf/model_parity/phase_gate_latest.md`
   - 판정 산출물(`precheck`, `phase별 reached/match/error`, `first failure by step`)은 `perf/model_parity/phase_gate_latest.md`를 기준으로 확인한다.

추가 실행 원칙:

- Java trace exporter는 `RecursiveGraphLayoutEngine`을 우선 사용한다(실패 시 layered provider fallback).
- Rust trace는 Java와 동일하게 루트 layout graph만 snapshot으로 기록한다(하위 recursive run은 미기록).

### 11.3 현재 기준선 (재측정 결과)

- 기준 모델(`java_status=ok`): `1439`
- 비교불가(error): `1`
  - Java만 trace 없음: `0`
  - Rust만 trace 없음: `1` (`realworld/ptolemy/hierarchical/ptides_powerplant_PowerPlant.elkt`)
  - 양측 trace 없음: `0`
- 비교 가능(shared): `1438`
- shared 모델 phase 결과: `all_match=0`, `diverged=1438`
- 최신 gate 요약: `perf/model_parity/phase_gate_latest.md`
- 최근 갱신(2026-02-22, recursive+strict): `step 8` error가 `41 -> 4`로 감소, `step 0` error는 `0` 유지
- 최신 step 범위(루트 trace 기준): `0..20`
- 현재 frontier(낮은 step) 실패 분포: `step1=82`, `step2=50`, `step3=10`, `step4=1`, `step7=1`, `step8=4`
- 1차 대형 hotspot: `step10=327`, `step11=567`, `step12=206`

### 11.4 단계별 처리 순서 (앞 phase 우선)

다음 규칙으로만 진행한다.

1. `Precheck` 비교불가 `0` 유지
2. `first_failure_by_step`에서 **가장 작은 step**의 `error`를 `0`으로 만든다.
3. 해당 step이 `0`이 되면, 다음으로 작은 step으로 이동한다.
4. 이미 더 작은 step에 `error>0`가 남아 있으면, 큰 step은 수정/평가하지 않는다.

현재 우선 순위(frontier):

1. `step 1~4` (가장 이른 phase frontier)
2. `step 7~12`
3. `step 13~20`

각 단계 완료 기준은 해당 step의 `error=0`이다.
