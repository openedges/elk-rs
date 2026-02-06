#!/usr/bin/env sh
set -eu

RUST_FILE="${1:-perf/results_layered_issue_scenarios.csv}"
JAVA_FILE="${2:-perf/java_results_layered_issue_scenarios.csv}"
WINDOW="${3:-3}"
BUFFER_PCT="${4:-2}"
OUT="${5:-perf/java_parity_thresholds.suggested.csv}"

RUST_SCENARIO_COL="${RUST_SCENARIO_COL:-2}"
RUST_AVG_COL="${RUST_AVG_COL:-6}"
RUST_OPS_COL="${RUST_OPS_COL:-7}"

JAVA_SCENARIO_COL="${JAVA_SCENARIO_COL:-2}"
JAVA_AVG_COL="${JAVA_AVG_COL:-6}"
JAVA_OPS_COL="${JAVA_OPS_COL:-7}"

if [ ! -f "$RUST_FILE" ]; then
  echo "missing rust file: $RUST_FILE" >&2
  exit 2
fi
if [ ! -f "$JAVA_FILE" ]; then
  echo "missing java file: $JAVA_FILE" >&2
  exit 2
fi

mkdir -p "$(dirname "$OUT")"

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/java-threshold-suggest.XXXXXX")"
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

rust_window="$tmp_dir/rust.window.tsv"
java_window="$tmp_dir/java.window.tsv"
joined="$tmp_dir/joined.tsv"

window_avg_by_scenario() {
  file=$1
  scenario_col=$2
  avg_col=$3
  ops_col=$4
  out=$5

  awk -F',' -v w="$WINDOW" -v sc="$scenario_col" -v ac="$avg_col" -v oc="$ops_col" '
    function is_header(v) {
      return (v == "scenario" || v == "avg_ms" || v == "scenarios_per_sec")
    }
    {
      scenario = $sc
      avg = $ac
      ops = $oc
      if (is_header(scenario) || scenario == "" || avg == "" || ops == "") {
        next
      }
      idx[scenario] += 1
      i = idx[scenario]
      avg_buf[scenario, i] = avg + 0.0
      ops_buf[scenario, i] = ops + 0.0
    }
    END {
      for (scenario in idx) {
        n = idx[scenario]
        start = n - w + 1
        if (start < 1) {
          start = 1
        }
        sum_avg = 0.0
        sum_ops = 0.0
        count = 0
        for (i = start; i <= n; i++) {
          sum_avg += avg_buf[scenario, i]
          sum_ops += ops_buf[scenario, i]
          count += 1
        }
        if (count > 0) {
          printf "%s\t%.12f\t%.12f\t%d\n", scenario, (sum_avg / count), (sum_ops / count), count
        }
      }
    }
  ' "$file" | sort > "$out"
}

window_avg_by_scenario "$RUST_FILE" "$RUST_SCENARIO_COL" "$RUST_AVG_COL" "$RUST_OPS_COL" "$rust_window"
window_avg_by_scenario "$JAVA_FILE" "$JAVA_SCENARIO_COL" "$JAVA_AVG_COL" "$JAVA_OPS_COL" "$java_window"

awk -F'\t' '
  NR == FNR {
    scenario = $1
    j_avg[scenario] = $2 + 0.0
    j_ops[scenario] = $3 + 0.0
    j_n[scenario] = $4 + 0
    next
  }
  {
    scenario = $1
    r_avg = $2 + 0.0
    r_ops = $3 + 0.0
    r_n = $4 + 0
    if (!(scenario in j_n) || j_n[scenario] == 0 || r_n == 0) {
      next
    }
    print scenario "\t" r_avg "\t" r_ops "\t" j_avg[scenario] "\t" j_ops[scenario]
  }
' "$java_window" "$rust_window" | sort > "$joined"

{
  echo "scenario,max_avg_ms_regression_pct,max_scenarios_per_sec_regression_pct"
  awk -F'\t' -v buffer="$BUFFER_PCT" '
    function round2(v) {
      return sprintf("%.2f", v + 0.0)
    }
    {
      scenario = $1
      r_avg = $2 + 0.0
      r_ops = $3 + 0.0
      j_avg = $4 + 0.0
      j_ops = $5 + 0.0

      avg_reg = 0.0
      ops_reg = 0.0

      if (j_avg > 0 && r_avg > j_avg) {
        avg_reg = ((r_avg - j_avg) / j_avg) * 100.0
      }
      if (j_ops > 0 && r_ops < j_ops) {
        ops_reg = ((j_ops - r_ops) / j_ops) * 100.0
      }

      avg_th = avg_reg + buffer
      ops_th = ops_reg + buffer
      if (avg_th < 0) {
        avg_th = 0
      }
      if (ops_th < 0) {
        ops_th = 0
      }

      if (avg_th > max_avg) {
        max_avg = avg_th
      }
      if (ops_th > max_ops) {
        max_ops = ops_th
      }

      rows[scenario] = scenario "," round2(avg_th) "," round2(ops_th)
      scenarios[++n] = scenario
    }
    END {
      for (i = 1; i <= n; i++) {
        s = scenarios[i]
        print rows[s]
      }
      if (n > 0) {
        print "*," round2(max_avg) "," round2(max_ops)
      }
    }
  ' "$joined"
} > "$OUT"

echo "wrote suggested thresholds: $OUT"
