#!/bin/sh
# ============================================================================
# cleanup_maven_cache.sh — Maven cache cleanup utility for ELK artifacts
#
# Standalone script (no dependency on elk_java_common.sh).
# Default mode is dry-run for safety.
#
# Usage:
#   sh scripts/java/cleanup_maven_cache.sh [OPTIONS]
#
# Options:
#   --dry-run        Preview deletions (default)
#   --apply          Actually delete files
#   --m2-repo PATH   Maven local repository path (default: ~/.m2/repository)
#   --snapshots      Only *-SNAPSHOT directories under org.eclipse.elk (default)
#   --all-elk        All org.eclipse.elk artifacts
#   --tycho-cache    Also include Tycho p2/cache directories
#   --help           Show this help
# ============================================================================
set -eu

# Defaults
MODE=dry-run
M2_REPO="${HOME}/.m2/repository"
SCOPE=snapshots

usage() {
  sed -n '/^# Usage:/,/^# ====/{ /^# ====/d; s/^# \{0,1\}//; p; }' "$0"
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run)    MODE=dry-run ;;
    --apply)      MODE=apply ;;
    --m2-repo)    shift; M2_REPO="$1" ;;
    --snapshots)  SCOPE=snapshots ;;
    --all-elk)    SCOPE=all-elk ;;
    --tycho-cache) SCOPE=tycho-cache ;;
    --help|-h)    usage ;;
    *)
      echo "unknown option: $1" >&2
      echo "run with --help for usage" >&2
      exit 1
      ;;
  esac
  shift
done

ELK_BASE="$M2_REPO/org/eclipse/elk"

# ---------- Collect targets -------------------------------------------------

targets=""
total_size=0

add_target() {
  _dir=$1
  [ -d "$_dir" ] || return 0
  targets="${targets:+$targets
}$_dir"
  # Portable size estimation (du -sk)
  _sz=$(du -sk "$_dir" 2>/dev/null | awk '{print $1}')
  total_size=$((total_size + _sz))
}

case "$SCOPE" in
  snapshots)
    if [ -d "$ELK_BASE" ]; then
      for d in $(find "$ELK_BASE" -maxdepth 2 -type d -name '*-SNAPSHOT' 2>/dev/null); do
        add_target "$d"
      done
    fi
    ;;
  all-elk)
    add_target "$ELK_BASE"
    ;;
  tycho-cache)
    # ELK snapshots
    if [ -d "$ELK_BASE" ]; then
      for d in $(find "$ELK_BASE" -maxdepth 2 -type d -name '*-SNAPSHOT' 2>/dev/null); do
        add_target "$d"
      done
    fi
    # Tycho p2 bundles
    if [ -d "$M2_REPO/p2" ]; then
      for d in $(find -L "$M2_REPO/p2" -maxdepth 4 -type d -name 'org.eclipse.elk*' 2>/dev/null); do
        add_target "$d"
      done
    fi
    # Tycho cache
    if [ -d "$M2_REPO/.cache/tycho" ]; then
      for d in $(find -L "$M2_REPO/.cache/tycho" -maxdepth 4 -type d -name 'org.eclipse.elk*' 2>/dev/null); do
        add_target "$d"
      done
    fi
    ;;
esac

# ---------- Report ----------------------------------------------------------

if [ -z "$targets" ]; then
  echo "no matching directories found (scope=$SCOPE, m2-repo=$M2_REPO)"
  exit 0
fi

count=$(printf '%s\n' "$targets" | wc -l | awk '{print $1}')
size_mb=$(awk "BEGIN { printf \"%.1f\", $total_size / 1024 }")

echo "scope: $SCOPE"
echo "m2-repo: $M2_REPO"
echo "found: $count directories (~${size_mb} MB)"
echo ""

printf '%s\n' "$targets" | while IFS= read -r d; do
  if [ "$MODE" = "apply" ]; then
    echo "  rm -rf $d"
    rm -rf "$d"
  else
    echo "  [dry-run] rm -rf $d"
  fi
done

echo ""
if [ "$MODE" = "apply" ]; then
  echo "deleted $count directories (~${size_mb} MB)"
else
  echo "dry-run complete. Use --apply to delete."
fi
