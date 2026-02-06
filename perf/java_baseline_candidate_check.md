# Java Baseline Candidate Check

- status: ready
- reason: candidate passed checks
- candidate: `perf/baselines/java_layered_issue_scenarios.candidate.csv`
- rust file: `perf/results_layered_issue_scenarios.csv`
- target baseline: `perf/baselines/java_layered_issue_scenarios.csv`
- require parity: `true`
- threshold: `0`

## Next Action

- promote candidate: `sh scripts/update_java_perf_baseline.sh "perf/baselines/java_layered_issue_scenarios.candidate.csv" "perf/baselines/java_layered_issue_scenarios.csv"`
