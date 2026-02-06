# Java Perf Status

- results report: `perf/java_vs_rust.md` (yes)
- baseline report: `perf/java_vs_rust_baseline.md` (yes)
- java results csv: `perf/java_results_layered_issue_scenarios.csv` (yes)
- java baseline csv: `perf/baselines/java_layered_issue_scenarios.csv` (yes)
- rust layered issue csv: `perf/results_layered_issue_scenarios.csv`
- java baseline candidate csv: `perf/baselines/java_layered_issue_scenarios.candidate.csv` (yes)
- java baseline candidate report: `perf/java_baseline_candidate_status.md` (yes)
- java baseline candidate check report: `perf/java_baseline_candidate_check.md` (yes)
- java baseline candidate check status: ready
- java baseline candidate equals baseline: yes
- results skip reason: none

## Next Action

- baseline is already synchronized with the ready candidate.
- re-run compare in `java_compare_mode=both` with desired parity gates when Rust perf inputs change.
