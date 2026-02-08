#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

TMP_OUTPUT="$(mktemp)"
TMP_AGENTS="$(mktemp)"
trap 'rm -f "${TMP_OUTPUT}" "${TMP_AGENTS}"' EXIT

cargo test -p org-eclipse-elk-alg-layered --test node_promotion_test \
  node_promotion_external_ptolemy_model_parse_coverage_if_available \
  -- --nocapture --test-threads=1 2>&1 | tee -a "${TMP_OUTPUT}"

cargo test -p org-eclipse-elk-alg-layered --test node_promotion_test \
  node_promotion_external_ptolemy_resources_model_order_if_available \
  -- --nocapture --test-threads=1 2>&1 | tee -a "${TMP_OUTPUT}"

parse_line="$(grep 'METRIC:ptolemy_parse_coverage' "${TMP_OUTPUT}" | tail -n1 || true)"
model_line="$(grep 'METRIC:ptolemy_model_order_validated' "${TMP_OUTPUT}" | tail -n1 || true)"

if [[ -z "${parse_line}" || -z "${model_line}" ]]; then
  echo "[update_ptolemy_coverage_agents] metric line parse failed" >&2
  exit 1
fi

parsed="$(echo "${parse_line}" | sed -E 's/.*parsed=([0-9]+).*/\1/')"
parse_sampled="$(echo "${parse_line}" | sed -E 's/.*sampled=([0-9]+).*/\1/')"
parse_coverage="$(echo "${parse_line}" | sed -E 's/.*coverage=([0-9]+\.[0-9]+).*/\1/')"

validated="$(echo "${model_line}" | sed -E 's/.*checked=([0-9]+).*/\1/')"
validated_sampled="$(echo "${model_line}" | sed -E 's/.*sampled=([0-9]+).*/\1/')"

latest_idx="$(grep -o '최신-[0-9]\+' AGENTS.md | sed -E 's/최신-//' | sort -n | tail -n1 || true)"
if [[ -z "${latest_idx}" ]]; then
  latest_idx=0
fi
next_idx=$((latest_idx + 1))

entry="- ptolemy coverage 자동 기록(최신-${next_idx}): auto script(bash scripts/update_ptolemy_coverage_agents.sh)로 external ptolemy parse coverage(parsed=${parsed}/${parse_sampled}, coverage=${parse_coverage})와 validated model 수(model-order checked=${validated}/${validated_sampled})를 배치별 정량 기록"

awk -v entry="${entry}" '
BEGIN { inserted = 0 }
/^## 진행률\(최신\)/ {
  if (!inserted) {
    print entry
    inserted = 1
  }
}
{ print }
END {
  if (!inserted) {
    print entry
  }
}
' AGENTS.md > "${TMP_AGENTS}"

mv "${TMP_AGENTS}" AGENTS.md

echo "[update_ptolemy_coverage_agents] ${entry}"
