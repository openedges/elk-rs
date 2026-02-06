Scripts overview:

- `run_perf_comment_attachment.sh [count] [iterations] [warmup] [output]`
- `run_perf_graph_validation.sh [nodes] [edges] [iterations] [warmup] [mode] [output]`
- `run_perf_recursive_layout.sh [nodes] [edges] [iterations] [warmup] [algorithm] [validate_graph] [validate_options] [output]`
- `run_perf_layered_layout.sh [nodes] [edges] [iterations] [warmup] [validate_graph] [validate_options] [output]`
- `run_perf_recursive_layout_scenarios.sh [scenarios] [iterations] [warmup] [output]` (`fixed_dense`, `fixed_sparse`, `random_dense`, `random_sparse`, `box_sparse`, `box_large`, `fixed_validated`, `random_validated`, `box_validated` preset 시나리오; scenarios 인자가 비어 있으면 `PERF_RECURSIVE_SCENARIO_PROFILE=quick|default|full`로 기본 세트 선택)
- `run_perf_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]`
- `run_java_perf_layered_issue_scenarios.sh [scenarios] [iterations] [warmup] [output]` (external ELK Java layered test 벤치 실행; benchmark test source는 저장소 내 템플릿에서 임시 주입 후 자동 정리)
- `run_perf_all.sh` (runs all perf scripts with defaults; supports env overrides)
- `compare_perf_results.sh [window]` (`PERF_COMPARE_MODE=window|baseline|both`, 기본 window; baseline 모드 시 `PERF_BASELINE_LAYERED_FILE` + `PERF_BASELINE_RECURSIVE_SCENARIOS_FILE` 기준 비교)
- `check_recursive_perf_runtime_budget.sh [results_file] [profile] [report]` (recursive scenario CSV의 최신 시나리오별 `avg_iteration_ms`가 profile budget(`quick|default|full`)을 넘는지 검사; 기본 budget은 `RECURSIVE_BUDGET_MS_QUICK=40`, `RECURSIVE_BUDGET_MS_DEFAULT=60`, `RECURSIVE_BUDGET_MS_FULL=120`, `RECURSIVE_RUNTIME_BUDGET_STRICT=true`면 초과 시 실패)
- `summarize_perf_results.sh [output]` (writes `perf/summary.md` by default)
- `check_perf_regression.sh [threshold] [window]` (`PERF_COMPARE_MODE=window|baseline|both`; baseline 모드 시 `PERF_BASELINE_LAYERED_FILE` + `PERF_BASELINE_RECURSIVE_SCENARIOS_FILE` 기준 회귀 판정)
- `update_perf_baseline.sh [source] [target]` (기본 `perf/results_layered_issue_scenarios.csv` -> `perf/baselines/layered_issue_scenarios.csv`)
- `update_perf_recursive_scenarios_baseline.sh [source] [target]` (기본 `perf/results_recursive_layout_scenarios.csv` -> `perf/baselines/recursive_layout_scenarios.csv`)
- Baseline 운영 규칙은 `perf/baselines/POLICY.md` 참고
- `run_perf_and_compare.sh [window] [mode]` (perf + compare + summary)
- `run_perf_and_check.sh [threshold] [window] [mode]` (perf + compare + summary + regression gate)
- `compare_java_perf_results.sh [rust_file] [java_file] [window] [output]` (layered issue 시나리오 기준 Java vs Rust 비교 리포트 생성)
- `check_java_perf_parity.sh [rust_file] [java_file] [window] [threshold]` (Java 대비 회귀 게이트; 기본 threshold 0%)
- `check_java_perf_parity_scenarios.sh [rust_file] [java_file] [window] [thresholds_file]` (시나리오별 Java parity 게이트; 기본 threshold 파일 `perf/java_parity_thresholds.csv`)
- `suggest_java_parity_thresholds.sh [rust_file] [java_file] [window] [buffer_pct] [output]` (최근 window 기준 Java 대비 Rust 회귀율에서 시나리오별 threshold 후보 CSV 생성; 기본 `perf/java_parity_thresholds.suggested.csv`)
- `apply_java_parity_thresholds.sh [source] [target]` (threshold 후보 CSV를 검증한 뒤 운영 CSV에 적용; 기본 `perf/java_parity_thresholds.suggested.csv` -> `perf/java_parity_thresholds.csv`)
- `check_java_perf_artifacts.sh [java_file] [report_file]` (Java compare 단계 CSV/리포트 산출물 검증 + 데이터 행 최소 개수/시나리오 커버리지 게이트; optional header 자동 스킵, 최소 행 기준은 `max(JAVA_ARTIFACT_MIN_ROWS, required_scenario_count)`)
- `update_java_perf_baseline.sh [source] [target]` (기본 `perf/java_results_layered_issue_scenarios.csv` -> `perf/baselines/java_layered_issue_scenarios.csv`)
- `summarize_java_perf_status.sh [results_report] [baseline_report] [java_results_file] [java_baseline_file] [output]` (Java compare 상태/다음 액션 요약 리포트 생성; 기본 `perf/java_perf_status.md`)
- `export_java_baseline_candidate.sh [source] [target] [report]` (Java 결과 CSV를 baseline 후보로 복사하고 상태 리포트 생성; 정책 검증 실패 시 `JAVA_CANDIDATE_STRICT`에 따라 fail/skip)
- `check_java_baseline_candidate.sh [candidate] [rust_file] [window] [threshold] [report]` (candidate 승격 가능 상태 점검: artifact 정책 + Rust 대비 compare/parity 검증, 기본 리포트 `perf/java_baseline_candidate_check.md`)
- `run_perf_and_compare_java.sh [java_file] [window] [threshold] [output]` (Rust layered issue perf 실행 + [선택] Java CSV 생성 + Java 비교/게이트; `JAVA_PERF_COMPARE_MODE=results|baseline|both`)
- `check_core_options_parity.sh [report]` (Java `CoreOptions.java`와 Rust `core_options.rs`/`core_options_meta.rs`를 비교해 option/category drift 및 non-qualified `set_category_id`를 검출; 기본 리포트 `perf/core_options_parity.md`)
- `check_core_option_dependency_parity.sh [report]` (Java `CoreOptions.java`와 Rust `core_options_meta.rs`의 `addDependency`/`add_dependency`를 source-target-value 기준으로 정량 비교; 기본 `perf/core_option_dependency_parity.md`, `CORE_OPTION_DEPENDENCY_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_id_parity.sh [report]` (Java/Rust `ALGORITHM_ID` 목록을 정량 비교해 누락/추가 알고리즘 ID를 리포트; 기본 `perf/algorithm_id_parity.md`, `ALGORITHM_ID_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_category_parity.sh [report]` (Java `*.Options.java`의 algorithm category와 Rust `set_category_id`를 정량 비교; 기본 `perf/algorithm_category_parity.md`, `ALGORITHM_CATEGORY_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_name_parity.sh [report]` (Java/Rust 알고리즘 메타데이터 `name` 문자열을 정량 비교; 기본 `perf/algorithm_name_parity.md`, `ALGORITHM_NAME_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_description_parity.sh [report]` (Java/Rust 알고리즘 메타데이터 `description` 문자열을 정량 비교; 기본 `perf/algorithm_description_parity.md`, `ALGORITHM_DESCRIPTION_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_option_support_parity.sh [report]` (Java `addOptionSupport`와 Rust `add_option_support`+core `add_known_option_default`를 알고리즘별 호출 수로 정량 비교; provider 등록이 있는 알고리즘은 core 중복 계수를 자동 제외, 기본 `perf/algorithm_option_support_parity.md`, `ALGORITHM_OPTION_SUPPORT_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_option_default_parity.sh [report]` (Java `addOptionSupport` default를 option-id 단위 mode(`explicit_null`/`explicit_nonnull`/`getDefault_*`)로 분해해 Rust `add_option_support`/`add_known_option_default` mode(`none`/`nonnull`)와 비교; `getDefault`는 Rust `none`/`nonnull` 모두 허용, explicit mismatch/unknown만 drift 판정, missing/extra pair는 informational로 리포트, provider 등록 알고리즘의 core 중복 계수는 자동 제외, 기본 `perf/algorithm_option_default_parity.md`, `ALGORITHM_OPTION_DEFAULT_PARITY_STRICT=true`면 drift 시 실패, `ALGORITHM_OPTION_DEFAULT_PARITY_KEEP_TMP=true`면 중간 TSV를 보존해 디버깅 가능)
- `check_algorithm_option_default_value_parity.sh [report]` (Java/Rust 옵션 정의 기본값을 option-id 단위로 정규화해 타입/상수 레벨(`null`/`bool`/`number`/`string`/`enum`) 동치성을 비교; Java `addOptionSupport`에 실제로 등장하는 option id만 대상으로 집계, 비교 가능한 mismatch만 drift 판정, 비교 불가 케이스는 informational로 리포트, 기본 `perf/algorithm_option_default_value_parity.md`, `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_STRICT=true`면 drift 시 실패, `ALGORITHM_OPTION_DEFAULT_VALUE_PARITY_KEEP_TMP=true`면 중간 TSV를 보존)
- `check_algorithm_feature_parity.sh [report]` (Java `supportedFeatures`와 Rust `add_supported_feature`를 알고리즘별 feature pair로 정량 비교; 기본 `perf/algorithm_feature_parity.md`, `ALGORITHM_FEATURE_PARITY_STRICT=true`면 drift 시 실패)
- `check_algorithm_metadata_parity.sh [report]` (Java `*.Options.java`의 algorithm metadata category/melkBundleName/definingBundleId/imagePath와 Rust `LayoutAlgorithmData` 등록값을 정량 비교; 기본 `perf/algorithm_metadata_parity.md`, `ALGORITHM_METADATA_PARITY_STRICT=true`면 drift 시 실패)
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
RECURSIVE_SCENARIO_PROFILE=default
RECURSIVE_SCENARIOS=
RECURSIVE_SCENARIO_ITERATIONS=5
RECURSIVE_SCENARIO_WARMUP=1
RECURSIVE_SCENARIO_OUTPUT=perf/results_recursive_layout_scenarios.csv
LAYERED_ISSUE_SCENARIOS=issue_405,issue_603,issue_680,issue_871,issue_905
LAYERED_ISSUE_ITERATIONS=20
LAYERED_ISSUE_WARMUP=3
LAYERED_ISSUE_OUTPUT=perf/results_layered_issue_scenarios.csv
```

CI workflows (GitHub Actions):
- `.github/workflows/ci.yml` runs `run_fast_checks.sh` on push/PR.
- `.github/workflows/perf.yml` runs perf scripts on manual dispatch and uploads CSV/summary artifacts.
- `.github/workflows/perf.yml`에서 `recursive_runtime_budget_gate=true`를 켜면 `check_recursive_perf_runtime_budget.sh`를 실행해 `perf/recursive_runtime_budget.md`를 생성하고 profile별 budget 초과를 gate로 차단한다.
- `.github/workflows/perf.yml`의 기본 Java 경로는 strict 운영값으로 고정되어 있다(`java_compare_enabled=true`, `java_compare_mode=both`, `java_generate_enabled=true`, `java_export_baseline_candidate=true`, `java_export_candidate_strict=true`, `java_parity_gate=true`, `java_baseline_parity_gate=true`).
- `.github/workflows/perf.yml` Java step은 `JAVA_PERF_EXTERNAL_ISOLATE=true`로 `external/elk`를 격리 실행한다.
- `.github/workflows/perf.yml` validates Java compare artifacts with `check_java_perf_artifacts.sh` when Java compare is enabled.
- `.github/workflows/perf.yml` generates parity reports `perf/core_options_parity.md`, `perf/core_option_dependency_parity.md`, `perf/algorithm_id_parity.md`, `perf/algorithm_category_parity.md`, `perf/algorithm_name_parity.md`, `perf/algorithm_description_parity.md`, `perf/algorithm_option_support_parity.md`, `perf/algorithm_option_default_parity.md`, `perf/algorithm_feature_parity.md`, `perf/algorithm_metadata_parity.md` as artifacts.
- `.github/workflows/perf.yml`에서 `java_generate_dry_run=true`이면 Java compare/parity는 건너뛰고 dry-run 요약 리포트(`perf/java_vs_rust.md`)만 남긴다.
- `.github/workflows/perf.yml`에서 `java_compare_mode=baseline|both`면 기준선 리포트(`perf/java_vs_rust_baseline.md`)와 baseline parity gate를 추가 수행한다.
- `.github/workflows/perf.yml`는 `recursive_scenarios` 입력으로 비-layered recursive 시나리오 perf를 수집한다(`perf/results_recursive_layout_scenarios.csv`).
- `.github/workflows/perf.yml`는 `java_parity_thresholds_file` 입력으로 Java parity 시나리오별 임계치 CSV를 주입한다.
- `.github/workflows/perf.yml` 입력 `java_artifact_min_rows`로 Java CSV 최소 데이터 행 수 게이트를 조정하며, 시나리오 커버리지는 `layered_issue_scenarios` 기준으로 검사한다(실제 최소 행 기준은 `max(java_artifact_min_rows, scenario_count)`).
- `.github/workflows/perf.yml` 입력 `java_skip_dns_check`/`java_required_hosts`로 Java generation DNS preflight 정책을 runner 환경별로 제어할 수 있다.
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
JAVA_PARITY_SCENARIO_THRESHOLDS_FILE=perf/java_parity_thresholds.csv
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
- `JAVA_PARITY_SCENARIO_THRESHOLDS_FILE`가 존재하면 Java parity gate는 `perf/java_parity_thresholds.csv` 같은 시나리오별 임계치 CSV를 사용하고, 파일이 없으면 단일 `THRESHOLD` 기준으로 자동 fallback한다.
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
