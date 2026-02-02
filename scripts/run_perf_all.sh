#!/bin/sh
set -eu

sh scripts/run_perf_comment_attachment.sh
sh scripts/run_perf_graph_validation.sh
sh scripts/run_perf_recursive_layout.sh
