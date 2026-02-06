Scripts overview:

- `run_perf_comment_attachment.sh [count] [iterations] [warmup] [output]`
- `run_perf_graph_validation.sh [nodes] [edges] [iterations] [warmup] [mode] [output]`
- `run_perf_recursive_layout.sh [nodes] [edges] [iterations] [warmup] [algorithm] [validate_graph] [validate_options] [output]`
- `run_perf_layered_layout.sh [nodes] [edges] [iterations] [warmup] [validate_graph] [validate_options] [output]`
- `run_perf_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]`
- `run_java_perf_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]` (external ELK Java layered test 벤치 실행; benchmark test source는 저장소 내 템플릿에서 임시 주입 후 자동 정리)
- `run_perf_all.sh` (runs all perf scripts with defaults; supports env overrides)
- `compare_perf_results.sh [window]` (`PERF_COMPARE_MODE=window|baseline|both`, 기본 window; baseline 모드 시 `PERF_BASELINE_LAYERED_FILE` 기준 비교)
- `summarize_perf_results.sh [output]` (writes `perf/summary.md` by default)
- `check_perf_regression.sh [threshold] [window]` (`PERF_COMPARE_MODE=window|baseline|both`; baseline 모드 시 `PERF_BASELINE_LAYERED_FILE` 기준 회귀 판정)
- `update_perf_baseline.sh [source] [target]` (기본 `perf/results_layered_issue_scenarios.csv` -> `perf/baselines/layered_issue_scenarios.csv`)
- Baseline 운영 규칙은 `perf/baselines/POLICY.md` 참고
- `run_perf_and_compare.sh [window] [mode]` (perf + compare + summary)
- `run_perf_and_check.sh [threshold] [window] [mode]` (perf + compare + summary + regression gate)
- `compare_java_perf_results.sh [rust_file] [java_file] [window] [output]` (layered issue 시나리오 기준 Java vs Rust 비교 리포트 생성)
- `check_java_perf_parity.sh [rust_file] [java_file] [window] [threshold]` (Java 대비 회귀 게이트; 기본 threshold 0%)
- `check_java_perf_artifacts.sh [java_file] [report_file]` (Java compare 단계 CSV/리포트 산출물 검증 + 데이터 행 최소 개수/시나리오 커버리지 게이트; optional header 자동 스킵, 최소 행 기준은 `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`)
- `update_java_perf_baseline.sh [source] [target]` (기본 `perf/java_results_layered_issue_scenarios.csv` -> `perf/baselines/java_layered_issue_scenarios.csv`)
- `summarize_java_perf_status.sh [results_report] [baseline_report] [java_results_file] [java_baseline_file] [output]` (Java compare 상태/다음 액션 요약 리포트 생성; 기본 `perf/java_perf_status.md`)
- `export_java_baseline_candidate.sh [source] [target] [report]` (Java 결과 CSV를 baseline 후보로 복사하고 상태 리포트 생성; 정책 검증 실패 시 `JAVA_CANDIDATE_STRICT`에 따라 fail/skip)
- `check_java_baseline_candidate.sh [candidate] [rust_file] [window] [threshold] [report]` (candidate 승격 가능 상태 점검: artifact 정책 + Rust 대비 compare/parity 검증, 기본 리포트 `perf/java_baseline_candidate_check.md`)
- `run_perf_and_compare_java.sh [java_file] [window] [threshold] [output]` (Rust layered issue perf 실행 + [선택] Java CSV 생성 + Java 비교/게이트; `JAVA_PERF_COMPARE_MODE=results|baseline|both`)
- `check_core_options_parity.sh [report]` (Java `CoreOptions.java`와 Rust `core_options.rs`/`core_options_meta.rs`를 비교해 option/category drift 및 non-qualified `set_category_id`를 검출; 기본 리포트 `perf/core_options_parity.md`)
- `run_all_checks.sh [threshold] [window]` (cargo test, clippy, perf gate)
- `run_fast_checks.sh` (cargo test, clippy only)

`run_perf_all.sh` env overrides (defaults shown):

```
COMMENT_COUNT=2000
COMMENT_ITERATIONS=5
COMMENT_WARMUP=1
COMMENT_OUTPUT=perf/results_comment_attachment.csv
GRAPH_NODES=1000
GRAPH_EDGES=2000
GRAPH_ITERATIONS=5
GRAPH_WARMUP=1
GRAPH_MODE=both
GRAPH_OUTPUT=perf/results_graph_validation.csv
LAYOUT_NODES=500
LAYOUT_EDGES=1000
LAYOUT_ITERATIONS=5
LAYOUT_WARMUP=1
LAYOUT_ALGORITHM=fixed
LAYOUT_VALIDATE_GRAPH=false
LAYOUT_VALIDATE_OPTIONS=false
LAYOUT_OUTPUT=perf/results_recursive_layout.csv
LAYOUT_LAYERED_OUTPUT=perf/results_recursive_layout_layered.csv
LAYERED_ISSUE_SCENARIOS=issue_405,issue_603,issue_680,issue_871,issue_905
LAYERED_ISSUE_ITERATIONS=20
LAYERED_ISSUE_WARMUP=3
LAYERED_ISSUE_OUTPUT=perf/results_layered_issue_scenarios.csv
```

CI workflows (GitHub Actions):
- `.github/workflows/ci.yml` runs `run_fast_checks.sh` on push/PR.
- `.github/workflows/perf.yml` runs perf scripts on manual dispatch and uploads CSV/summary artifacts.
- `.github/workflows/perf.yml`의 기본 Java 경로는 strict 운영값으로 고정되어 있다(`java_compare_enabled=true`, `java_compare_mode=both`, `java_generate_enabled=true`, `java_export_baseline_candidate=true`, `java_export_candidate_strict=true`, `java_parity_gate=true`, `java_baseline_parity_gate=true`).
- `.github/workflows/perf.yml` Java step은 `JAVA_PERF_EXTERNAL_ISOLATE=true`로 `external/elk`를 격리 실행한다.
- `.github/workflows/perf.yml` validates Java compare artifacts with `check_java_perf_artifacts.sh` when Java compare is enabled.
- `.github/workflows/perf.yml`에서 `java_generate_dry_run=true`이면 Java compare/parity는 건너뛰고 dry-run 요약 리포트(`perf/java_vs_rust.md`)만 남긴다.
- `.github/workflows/perf.yml`에서 `java_compare_mode=baseline|both`면 기준선 리포트(`perf/java_vs_rust_baseline.md`)와 baseline parity gate를 추가 수행한다.
- `.github/workflows/perf.yml` 입력 `java_artifact_min_rows`로 Java CSV 최소 데이터 행 수 게이트를 조정하며, 시나리오 커버리지는 `layered_issue_scenarios` 기준으로 검사한다(실제 최소 행 기준은 `max(java_artifact_min_rows, scenario_count)`).
- `.github/workflows/perf.yml`의 Java 단계는 `run_perf_and_compare_java.sh` 단일 파이프라인 호출로 통합되어, 로컬 실행과 CI 동작이 동일하다.
- `.github/workflows/perf.yml`는 Java 파이프라인 후 `perf/java_perf_status.md`를 생성해 결과/skip 상태와 baseline 업데이트 다음 액션을 artifact로 남긴다(후보 파일/리포트 상태 포함).
- `.github/workflows/perf.yml`에서 `java_export_baseline_candidate=true`면 `perf/java_baseline_candidate_status.md`와 `java_baseline_candidate_file` CSV를 artifact로 함께 남긴다.
- `.github/workflows/perf.yml`는 `java_export_baseline_candidate=true`일 때 `check_java_baseline_candidate.sh`를 실행해 `perf/java_baseline_candidate_check.md`를 생성하고 artifact로 업로드한다(`java_export_candidate_strict`가 strict 실패 정책을 공유).
- Java 실패 triage 절차는 `perf/JAVA_PERF_TRIAGE.md` 참고.

Java perf generation env overrides:

```
JAVA_PERF_GENERATE=false
JAVA_PERF_SCENARIOS=$LAYERED_ISSUE_SCENARIOS
JAVA_PERF_ITERATIONS=$LAYERED_ISSUE_ITERATIONS
JAVA_PERF_WARMUP=$LAYERED_ISSUE_WARMUP
JAVA_PERF_OUTPUT=$JAVA_FILE
JAVA_PERF_RESET_OUTPUT=true
JAVA_PERF_DRY_RUN=false
JAVA_PERF_VERIFY_ARTIFACTS=true
JAVA_PERF_ALLOW_GENERATE_FAILURE=false
JAVA_PERF_RETRIES=0
JAVA_PERF_RETRY_DELAY_SECS=3
JAVA_PERF_COMPARE_MODE=results
JAVA_BASELINE_FILE=perf/baselines/java_layered_issue_scenarios.csv
JAVA_BASELINE_OUTPUT=perf/java_vs_rust_baseline.md
JAVA_BASELINE_THRESHOLD=$THRESHOLD
JAVA_RESULTS_PARITY_GATE=true
JAVA_BASELINE_PARITY_GATE=true
JAVA_ARTIFACT_MIN_ROWS=1
JAVA_ARTIFACT_REQUIRED_SCENARIOS=$LAYERED_ISSUE_SCENARIOS
JAVA_PERF_MVN_BIN=mvn
JAVA_PERF_BUILD_PLUGINS=true
JAVA_PERF_EXTERNAL_ELK_ROOT=external/elk
JAVA_PERF_EXTERNAL_ISOLATE=true
JAVA_PERF_EXTERNAL_WORKTREE_ROOT=/tmp
JAVA_PERF_PREPARE_POM=<auto:$JAVA_PERF_EXTERNAL_ELK_ROOT/build/pom.xml or isolated worktree>
JAVA_PERF_PREPARE_MODULES=
JAVA_PERF_TEST_POM=<auto:$JAVA_PERF_EXTERNAL_ELK_ROOT/build/pom.xml or isolated worktree>
JAVA_PERF_TEST_MODULES=../test/org.eclipse.elk.alg.test,../test/org.eclipse.elk.alg.layered.test
JAVA_PERF_TEST_CLASS=LayeredIssuePerfBenchTest
JAVA_PERF_TEST_METHOD=
JAVA_PERF_TEST_GOAL=verify
JAVA_PERF_BENCH_INJECT=true
JAVA_PERF_BENCH_SOURCE=scripts/java/LayeredIssuePerfBenchTest.java
JAVA_PERF_BENCH_DEST=<auto:$JAVA_PERF_EXTERNAL_ELK_ROOT/test/... or isolated worktree>
JAVA_PERF_BENCH_CLEANUP=true
JAVA_PERF_PREPARE_ARGS="-DskipTests -DskipITs"
JAVA_PERF_MVN_LOCAL_REPO=
JAVA_PERF_MVN_ARGS=
JAVA_PERF_SKIP_DNS_CHECK=false
JAVA_PERF_REQUIRED_HOSTS=repo.eclipse.org,repo.maven.apache.org
```

참고:
- `JAVA_ARTIFACT_MIN_ROWS`가 작게 설정되어 있어도 `JAVA_ARTIFACT_REQUIRED_SCENARIOS`에 지정된 시나리오 개수보다 낮게 내려가지는 않는다.
- `run_perf_and_compare_java.sh`에서 `JAVA_PERF_GENERATE=true`이고 `JAVA_PERF_MVN_LOCAL_REPO`가 비어 있으면 lock 충돌 회피를 위해 per-run 임시 경로(`${TMPDIR:-/tmp}/m2-java-perf-${USER:-user}-$$`)를 자동 사용한다.
- `JAVA_PERF_COMPARE_MODE=baseline`이면 `JAVA_BASELINE_FILE`만으로 비교/게이트를 수행하며 Java CSV 생성 단계는 선택사항이다.
- `JAVA_PERF_COMPARE_MODE=baseline`에서는 `JAVA_PERF_GENERATE=true`여도 wrapper가 Java 생성 단계를 자동 skip한다.
- `LAYERED_ISSUE_SKIP_RUST_RUN=true`를 주면 Rust layered perf 재실행을 건너뛰고 기존 `LAYERED_ISSUE_OUTPUT` 파일로 Java compare만 수행한다(CI 통합 단계에서 사용).
- `JAVA_PERF_ALLOW_GENERATE_FAILURE=true`면 Java 생성 실패 시 wrapper가 results compare를 skip report로 대체하고(`both` 모드면 baseline compare는 계속 수행) 종료 코드를 유지한다.
- `JAVA_PERF_RETRIES`/`JAVA_PERF_RETRY_DELAY_SECS`로 Java Maven 호출 재시도 정책을 조정할 수 있다.
- `run_java_perf_layered_issue_scenarios.sh`는 기본적으로 DNS preflight를 수행해 `repo.eclipse.org`, `repo.maven.apache.org` 해석 실패 시 Maven 실행 전에 즉시 중단한다(`JAVA_PERF_SKIP_DNS_CHECK=true`로 우회 가능).
- `run_java_perf_layered_issue_scenarios.sh`는 기본적으로 `external/elk`를 임시 격리 디렉터리(`JAVA_PERF_EXTERNAL_ISOLATE=true`)에서 실행한다(가능하면 git worktree, 실패 시 임시 복제본 fallback).
- 기본값 기준으로 실행 후 원본 `external/elk` 워크트리는 변경되지 않는다(원본에 직접 실행하려면 `JAVA_PERF_EXTERNAL_ISOLATE=false`).
- baseline 후보 export/ready check는 `JAVA_CANDIDATE_MIN_ROWS`, `JAVA_CANDIDATE_REQUIRED_SCENARIOS`, `JAVA_CANDIDATE_REQUIRE_PARITY`, `JAVA_CANDIDATE_STRICT`로 검증/실패 정책을 조정할 수 있다.

반복 실행 팁:
- 첫 실행에서 `JAVA_PERF_BUILD_PLUGINS=true`로 local Maven/Tycho artifact를 준비한 뒤,
- 이후 반복 실행은 `JAVA_PERF_BUILD_PLUGINS=false`로 test 단계만 실행하면 훨씬 빠르게 Java CSV를 갱신할 수 있다.
- CI에서는 `JAVA_PERF_MVN_LOCAL_REPO`를 run 단위 임시 디렉터리로 지정해 Tycho lock 충돌을 피한다
  (예: `${RUNNER_TEMP}/m2-java-perf-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}`).

Java perf CI 운영 가이드(확정):
- `java_generate_enabled=true`를 켜는 워크플로는 `JAVA_PERF_MVN_LOCAL_REPO`를 반드시 run-attempt 단위로 분리한다.
- 동일 job 내 prepare/install 단계와 test 단계는 같은 `JAVA_PERF_MVN_LOCAL_REPO`를 사용해 artifact 재활용률을 유지한다.
- 동시 실행되는 다른 job/runner와 같은 경로를 공유하지 않는다(공유 `.m2` 사용 금지).
- 같은 run에서 Java CSV만 다시 만들 때는 `JAVA_PERF_BUILD_PLUGINS=false`로 전환해 lock 대기 시간을 줄인다.
