#!/usr/bin/env sh
set -eu

APPLY=false
INCLUDE_TRACKED=false
ROOT="parity"

usage() {
  cat <<'EOF'
Usage:
  sh scripts/clean_parity_temp.sh [--apply] [--include-tracked] [--root <parity_dir>]

Options:
  --apply            Actually delete files/directories (default is dry-run).
  --include-tracked  Also delete paths tracked by git (legacy cleanup).
  --root <dir>       Parity directory root (default: parity).
  -h, --help         Show this help.

Notes:
  - Default mode is safe: tracked files are never deleted.
  - Use --include-tracked only when intentionally cleaning legacy tracked runtime artifacts.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --apply)
      APPLY=true
      ;;
    --include-tracked|--all)
      INCLUDE_TRACKED=true
      ;;
    --root)
      shift
      if [ "$#" -eq 0 ]; then
        echo "missing value for --root" >&2
        exit 1
      fi
      ROOT="$1"
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
  shift
done

if ! REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null); then
  echo "not inside a git repository" >&2
  exit 1
fi
cd "$REPO_ROOT"

if [ ! -d "$ROOT" ]; then
  echo "parity root does not exist: $ROOT" >&2
  exit 1
fi

if [ "$APPLY" = "true" ]; then
  echo "[clean] apply mode"
else
  echo "[clean] dry-run mode (add --apply to delete)"
fi

if [ "$INCLUDE_TRACKED" = "true" ]; then
  echo "[clean] include tracked paths: true"
else
  echo "[clean] include tracked paths: false"
fi

is_tracked_file() {
  path="$1"
  git ls-files --error-unmatch -- "$path" >/dev/null 2>&1
}

has_tracked_under() {
  dir="$1"
  if [ ! -d "$dir" ]; then
    return 1
  fi
  tracked_rows=$(git ls-files -- "$dir")
  [ -n "$tracked_rows" ]
}

remove_path() {
  path="$1"
  [ -e "$path" ] || return 0

  if [ "$INCLUDE_TRACKED" != "true" ]; then
    if [ -d "$path" ] && has_tracked_under "$path"; then
      echo "skip tracked dir : $path"
      return 0
    fi
    if [ ! -d "$path" ] && is_tracked_file "$path"; then
      echo "skip tracked file: $path"
      return 0
    fi
  fi

  echo "remove           : $path"
  if [ "$APPLY" = "true" ]; then
    rm -rf -- "$path"
  fi
}

remove_glob() {
  pattern="$1"
  for path in $pattern; do
    [ -e "$path" ] || continue
    remove_path "$path"
  done
}

remove_glob "$ROOT/tmp/*"
remove_path "$ROOT/test_parity"
remove_path "$ROOT/model_parity_categories"
remove_glob "$ROOT/layered_phase_wiring/*.tsv"

for parity_root in "$ROOT"/model_parity "$ROOT"/model_parity_*; do
  [ -d "$parity_root" ] || continue
  remove_path "$parity_root/java/input"
  remove_path "$parity_root/java/layout"
  remove_path "$parity_root/rust/layout"
  remove_path "$parity_root/rust"
  remove_path "$parity_root/java_trace"
  remove_path "$parity_root/rust_trace"
done

echo "[clean] done"
