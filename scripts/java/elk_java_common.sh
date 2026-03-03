#!/bin/sh
# ============================================================================
# elk_java_common.sh — shared functions for Java ELK build/test scripts
#
# POSIX sh compatible.  Caller must set EJC_PREFIX before sourcing:
#   EJC_PREFIX=JAVA_PARITY   (or JAVA_TRACE, etc.)
#   . "$SCRIPT_DIR/java/elk_java_common.sh"
#
# Every function reads ${EJC_PREFIX}_FOO environment variables via eval-based
# indirection, so the same library works for all scripts.
# ============================================================================

# Double-load guard
if [ "${_EJC_LOADED:-}" = "true" ]; then
  return 0 2>/dev/null || true
fi
_EJC_LOADED=true

# ---------- internal helpers ------------------------------------------------

# ejc_resolve_var SUFFIX DEFAULT
#   Reads ${EJC_PREFIX}_${SUFFIX}; returns DEFAULT if unset/empty.
#   Result is stored in _ejc_val.
ejc_resolve_var() {
  eval "_ejc_val=\${${EJC_PREFIX}_${1}:-${2}}"
}

# ejc_resolve_var_raw SUFFIX
#   Like ejc_resolve_var but returns empty string (no default).
ejc_resolve_var_raw() {
  eval "_ejc_val=\${${EJC_PREFIX}_${1}:-}"
}

# _ejc_compute_build_key [GIT_ROOT]
#   Computes a build cache key from the git commit hash and patches checksum.
#   GIT_ROOT defaults to $EJC_ELK_ROOT.  Result is stored in _ejc_val.
_ejc_compute_build_key() {
  _bk_root=${1:-$EJC_ELK_ROOT}
  _bk_commit=$(git -C "$_bk_root" rev-parse HEAD 2>/dev/null || echo "unknown")

  ejc_resolve_var_raw PATCHES_DIR
  _bk_pdir=$_ejc_val

  _bk_psum="no-patches"
  if [ -d "$_bk_pdir" ]; then
    _bk_has_patches=false
    for _bk_f in "$_bk_pdir"/*.patch; do
      [ -f "$_bk_f" ] && _bk_has_patches=true && break
    done
    if [ "$_bk_has_patches" = "true" ]; then
      if command -v shasum >/dev/null 2>&1; then
        _bk_psum=$(cat "$_bk_pdir"/*.patch | shasum -a 256 | awk '{print $1}')
      elif command -v sha256sum >/dev/null 2>&1; then
        _bk_psum=$(cat "$_bk_pdir"/*.patch | sha256sum | awk '{print $1}')
      else
        _bk_psum=$(cat "$_bk_pdir"/*.patch | cksum | awk '{print $1}')
      fi
    fi
  fi

  _ejc_val="${_bk_commit}:${_bk_psum}"
}

# ---------- path helpers ----------------------------------------------------

# ejc_resolve_to_absolute PATH BASE
#   Converts a possibly-relative PATH to absolute using BASE as the prefix.
#   Result in _ejc_val.
ejc_resolve_to_absolute() {
  case "$1" in
    /*) _ejc_val="$1" ;;
    *)  _ejc_val="$2/$1" ;;
  esac
}

# ---------- validation ------------------------------------------------------

# ejc_validate_maven
#   Validates that the Maven binary (${EJC_PREFIX}_MVN_BIN) exists and is
#   executable.  Skipped when ${EJC_PREFIX}_DRY_RUN=true.
ejc_validate_maven() {
  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  ejc_resolve_var MVN_BIN mvn
  _mvn_bin=$_ejc_val

  case "$_mvn_bin" in
    */*)
      if [ ! -x "$_mvn_bin" ]; then
        echo "maven command is not executable: $_mvn_bin" >&2
        exit 1
      fi
      ;;
    *)
      if ! command -v "$_mvn_bin" >/dev/null 2>&1; then
        echo "missing maven command in PATH: $_mvn_bin" >&2
        exit 1
      fi
      ;;
  esac
}

# ejc_validate_integer NAME VALUE
#   Exits with error if VALUE is not a non-negative integer.
ejc_validate_integer() {
  case "$2" in
    ''|*[!0-9]*)
      echo "invalid $1 (must be non-negative integer): $2" >&2
      exit 1
      ;;
  esac
}

# ---------- clean external/elk guard ----------------------------------------

# ejc_check_clean_elk
#   Verifies that the external ELK tree has no local changes.
#   Reads ${EJC_PREFIX}_REQUIRE_CLEAN_EXTERNAL_ELK and
#   ${EJC_PREFIX}_EXTERNAL_ELK_ROOT.
#   Skipped when ${EJC_PREFIX}_DRY_RUN=true.
ejc_check_clean_elk() {
  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  ejc_resolve_var REQUIRE_CLEAN_EXTERNAL_ELK true
  [ "$_ejc_val" != "true" ] && return 0

  ejc_resolve_var_raw EXTERNAL_ELK_ROOT
  _elk_root=$_ejc_val

  if ! git -C "$_elk_root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "${EJC_PREFIX}_REQUIRE_CLEAN_EXTERNAL_ELK=true but external ELK root is not a git worktree: $_elk_root" >&2
    echo "set ${EJC_PREFIX}_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi

  _dirty_status=$(git -C "$_elk_root" status --porcelain 2>/dev/null || true)
  if [ -n "$_dirty_status" ]; then
    echo "external ELK tree has local changes; refusing to proceed to protect external/elk state." >&2
    printf "%s\n" "$_dirty_status" | sed -n '1,20p' >&2
    _dirty_lines=$(printf "%s\n" "$_dirty_status" | wc -l | awk '{print $1}')
    if [ "$_dirty_lines" -gt 20 ]; then
      echo "... (showing first 20 of $_dirty_lines changed paths)" >&2
    fi
    echo "set ${EJC_PREFIX}_REQUIRE_CLEAN_EXTERNAL_ELK=false to bypass this guard." >&2
    exit 1
  fi
}

# ---------- isolation (worktree / copy) -------------------------------------

# Internal state — set by ejc_create_isolation, used by ejc_cleanup_isolation.
_ejc_isolation_mode=none
_ejc_isolated_worktree_dir=
_ejc_isolated_copy_dir=
_ejc_isolation_reused=false
_ejc_isolation_persistent=false
EJC_BUILD_WAS_SKIPPED=false

# ejc_create_isolation INFIX
#   Creates a git worktree (preferred) or full copy of the external ELK tree
#   in a temporary directory.  INFIX is used in the temp-dir name (e.g.
#   "parity", "trace", "bench").
#   On success, sets EJC_ELK_ROOT to the isolated path.
#   Reads ${EJC_PREFIX}_DRY_RUN, ${EJC_PREFIX}_EXTERNAL_ISOLATE,
#   ${EJC_PREFIX}_EXTERNAL_ELK_ROOT, ${EJC_PREFIX}_EXTERNAL_WORKTREE_ROOT,
#   ${EJC_PREFIX}_ISOLATION_DIR.
ejc_create_isolation() {
  _infix=${1:-java}

  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  ejc_resolve_var EXTERNAL_ISOLATE true
  [ "$_ejc_val" != "true" ] && return 0

  ejc_resolve_var_raw EXTERNAL_ELK_ROOT
  _elk_src=$_ejc_val

  ejc_resolve_var EXTERNAL_WORKTREE_ROOT "${TMPDIR:-/tmp}"
  _wt_root=$_ejc_val

  # --- Check for persistent isolation mode ---
  ejc_resolve_var_raw ISOLATION_DIR
  _iso_dir=$_ejc_val

  if [ -n "$_iso_dir" ]; then
    _ejc_isolation_persistent=true

    # Compute build key from the original source
    _ejc_compute_build_key "$_elk_src"
    _current_key=$_ejc_val

    # Check if cached isolation is still valid
    if [ -d "$_iso_dir" ] && [ -f "$_iso_dir/.ejc-build-marker" ]; then
      _cached_key=$(cat "$_iso_dir/.ejc-build-marker")
      if [ "$_cached_key" = "$_current_key" ]; then
        EJC_ELK_ROOT=$_iso_dir
        _ejc_isolation_reused=true
        _ejc_isolated_worktree_dir=$_iso_dir
        echo "ejc: reusing cached isolation at: $_iso_dir (build key match)"
        return 0
      fi
      echo "ejc: cached isolation invalidated (key mismatch); recreating"
    fi

    # Remove stale isolation directory
    if [ -d "$_iso_dir" ]; then
      git -C "$_elk_src" worktree remove --force "$_iso_dir" >/dev/null 2>&1 || true
      rm -rf "$_iso_dir"
    fi

    # Create worktree (or copy) at the persistent path
    if git -C "$_elk_src" worktree add --detach "$_iso_dir" HEAD >/dev/null 2>&1; then
      EJC_ELK_ROOT=$_iso_dir
      _ejc_isolation_mode=worktree
      _ejc_isolated_worktree_dir=$_iso_dir
    else
      mkdir -p "$_iso_dir"
      cp -R "$_elk_src"/. "$_iso_dir"/
      EJC_ELK_ROOT=$_iso_dir
      _ejc_isolation_mode=copy
      _ejc_isolated_copy_dir=$_iso_dir
      echo "warning: failed to create git worktree; using copied external/elk tree at: $_iso_dir" >&2
    fi
    return 0
  fi

  # --- Standard ephemeral isolation (existing behavior) ---
  _ejc_isolated_worktree_dir=$(mktemp -d "$_wt_root/elk-java-${_infix}-worktree.XXXXXX")
  if [ -d "$_ejc_isolated_worktree_dir" ]; then
    rmdir "$_ejc_isolated_worktree_dir"
  fi

  if git -C "$_elk_src" worktree add --detach "$_ejc_isolated_worktree_dir" HEAD >/dev/null 2>&1; then
    EJC_ELK_ROOT=$_ejc_isolated_worktree_dir
    _ejc_isolation_mode=worktree
  else
    rm -rf "$_ejc_isolated_worktree_dir"
    _ejc_isolated_worktree_dir=
    _ejc_isolated_copy_dir=$(mktemp -d "$_wt_root/elk-java-${_infix}-copy.XXXXXX")
    cp -R "$_elk_src"/. "$_ejc_isolated_copy_dir"/
    EJC_ELK_ROOT=$_ejc_isolated_copy_dir
    _ejc_isolation_mode=copy
    echo "warning: failed to create git worktree; using copied external/elk tree at: $_ejc_isolated_copy_dir" >&2
  fi
}

# ejc_cleanup_isolation
#   Removes the isolation worktree/copy created by ejc_create_isolation.
#   Persistent isolation directories (ISOLATION_DIR) are preserved.
ejc_cleanup_isolation() {
  # Persistent isolation directories are preserved across runs
  if [ "$_ejc_isolation_persistent" = "true" ]; then
    return 0
  fi

  ejc_resolve_var_raw EXTERNAL_ELK_ROOT
  _elk_src=$_ejc_val

  if [ "$_ejc_isolation_mode" = "worktree" ] && [ -n "$_ejc_isolated_worktree_dir" ]; then
    git -C "$_elk_src" worktree remove --force "$_ejc_isolated_worktree_dir" >/dev/null 2>&1 || true
  fi
  if [ -n "$_ejc_isolated_worktree_dir" ]; then
    rm -rf "$_ejc_isolated_worktree_dir"
  fi
  if [ -n "$_ejc_isolated_copy_dir" ]; then
    rm -rf "$_ejc_isolated_copy_dir"
  fi
}

# ---------- trap / cleanup --------------------------------------------------

# Internal: caller-provided cleanup function name.
_ejc_extra_cleanup_fn=

# ejc_register_cleanup [EXTRA_FN]
#   Sets up EXIT/INT/TERM traps.  The trap calls EXTRA_FN (if provided) first,
#   then ejc_cleanup_isolation.  Call this once from the main script.
ejc_register_cleanup() {
  _ejc_extra_cleanup_fn=${1:-}

  # Define the actual trap function — must be a simple name for trap.
  _ejc_trap_handler() {
    if [ -n "$_ejc_extra_cleanup_fn" ]; then
      "$_ejc_extra_cleanup_fn" || true
    fi
    ejc_cleanup_isolation
  }

  trap _ejc_trap_handler EXIT INT TERM
}

# ---------- clean targets ---------------------------------------------------

# ejc_clean_all_targets
#   Removes all target/ directories and the build marker under EJC_ELK_ROOT.
#   Used by CLEAN_BUILD and available for direct invocation.
ejc_clean_all_targets() {
  if [ -z "$EJC_ELK_ROOT" ]; then
    echo "ejc: EJC_ELK_ROOT not set; cannot clean targets" >&2
    return 1
  fi

  for _td in "$EJC_ELK_ROOT"/plugins/*/target "$EJC_ELK_ROOT"/test/*/target; do
    if [ -d "$_td" ]; then
      rm -rf "$_td"
    fi
  done

  rm -f "$EJC_ELK_ROOT/.ejc-build-marker"
}

# ---------- patch application -----------------------------------------------

# ejc_apply_patches
#   Applies all *.patch files from ${EJC_PREFIX}_PATCHES_DIR to EJC_ELK_ROOT.
#   Skipped when ${EJC_PREFIX}_DRY_RUN=true or ${EJC_PREFIX}_APPLY_PATCHES=false.
#   Verifies known patches landed correctly.
ejc_apply_patches() {
  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  # Skip if isolation was reused (patches already applied)
  [ "$_ejc_isolation_reused" = "true" ] && return 0

  ejc_resolve_var APPLY_PATCHES true
  [ "$_ejc_val" != "true" ] && return 0

  ejc_resolve_var_raw PATCHES_DIR
  _patches_dir=$_ejc_val

  [ -d "$_patches_dir" ] || return 0

  for _p in "$_patches_dir"/*.patch; do
    [ -f "$_p" ] || continue
    if git -C "$EJC_ELK_ROOT" apply "$_p"; then
      echo "ejc: applied patch $(basename "$_p")"
      # Verify known patches
      case "$(basename "$_p")" in
        *self-loop-routing*)
          _shj="$EJC_ELK_ROOT/plugins/org.eclipse.elk.alg.layered/src/org/eclipse/elk/alg/layered/intermediate/loops/SelfHyperLoop.java"
          if [ -f "$_shj" ] && grep -q "MultimapBuilder" "$_shj"; then
            echo "ejc: VERIFIED patch content in SelfHyperLoop.java"
          else
            echo "ejc: WARNING -- patch applied but MultimapBuilder NOT found in SelfHyperLoop.java" >&2
          fi
          ;;
      esac
    else
      echo "ejc: failed to apply patch $(basename "$_p")" >&2
      exit 1
    fi
  done
}

# ---------- SNAPSHOT cache purge --------------------------------------------

# ejc_purge_snapshot_cache
#   Removes stale ELK *-SNAPSHOT directories from the local Maven cache.
#   Reads ${EJC_PREFIX}_DRY_RUN, ${EJC_PREFIX}_MVN_LOCAL_REPO.
ejc_purge_snapshot_cache() {
  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  # Skip purge when isolation was reused (build will also be skipped)
  [ "$_ejc_isolation_reused" = "true" ] && return 0

  # Skip purge when build was already skipped (e.g. BUILD_PLUGINS=false)
  [ "$EJC_BUILD_WAS_SKIPPED" = "true" ] && return 0

  ejc_resolve_var_raw MVN_LOCAL_REPO
  _elk_m2_base="${_ejc_val:-$HOME/.m2/repository}/org/eclipse/elk"

  [ -d "$_elk_m2_base" ] || return 0

  _stale_dirs=$(find "$_elk_m2_base" -maxdepth 2 -type d -name '*-SNAPSHOT' 2>/dev/null || true)
  [ -n "$_stale_dirs" ] || return 0

  _count=$(printf '%s\n' "$_stale_dirs" | wc -l | awk '{print $1}')
  echo "ejc: purging $_count stale ELK SNAPSHOT directories from Maven cache"
  printf '%s\n' "$_stale_dirs" | while IFS= read -r _d; do
    echo "  rm -rf $_d"
    rm -rf "$_d"
  done
}

# ---------- command runner (dry-run + retry) --------------------------------

# ejc_run_cmd CMD...
#   Executes CMD with dry-run and retry support.
#   Reads ${EJC_PREFIX}_DRY_RUN, ${EJC_PREFIX}_RETRIES,
#   ${EJC_PREFIX}_RETRY_DELAY_SECS.
ejc_run_cmd() {
  ejc_resolve_var DRY_RUN false
  if [ "$_ejc_val" = "true" ]; then
    printf "ejc dry-run:"
    for _arg in "$@"; do
      printf " %s" "$_arg"
    done
    printf "\n"
    return
  fi

  ejc_resolve_var RETRIES 0
  _retries=$_ejc_val
  ejc_resolve_var RETRY_DELAY_SECS 3
  _delay=$_ejc_val

  _attempt=0
  _max_attempts=$((_retries + 1))
  while [ "$_attempt" -lt "$_max_attempts" ]; do
    if "$@"; then
      return 0
    fi
    _attempt=$((_attempt + 1))
    if [ "$_attempt" -lt "$_max_attempts" ]; then
      echo "ejc: command failed (attempt $_attempt/$_max_attempts); retrying in ${_delay}s..." >&2
      if [ "$_delay" -gt 0 ]; then
        sleep "$_delay"
      fi
    fi
  done

  echo "ejc: command failed after $_max_attempts attempt(s)." >&2
  return 1
}

# ---------- Maven build -----------------------------------------------------

# ejc_mvn_build_plugins [POM]
#   Runs Maven install to build ELK plugins.  POM defaults to
#   $EJC_ELK_ROOT/build/pom.xml.
#   Supports build-cache markers: skips rebuild when the marker matches
#   the current build key (git commit + patches checksum).
#   Reads ${EJC_PREFIX}_BUILD_PLUGINS, ${EJC_PREFIX}_MVN_BIN,
#   ${EJC_PREFIX}_PREPARE_MODULES, ${EJC_PREFIX}_MVN_LOCAL_REPO,
#   ${EJC_PREFIX}_PREPARE_ARGS, ${EJC_PREFIX}_MVN_ARGS,
#   ${EJC_PREFIX}_FORCE_REBUILD, ${EJC_PREFIX}_CLEAN_BUILD.
#   Sets EJC_BUILD_WAS_SKIPPED=true/false.
ejc_mvn_build_plugins() {
  _pom=${1:-$EJC_ELK_ROOT/build/pom.xml}
  EJC_BUILD_WAS_SKIPPED=false

  ejc_resolve_var BUILD_PLUGINS true
  if [ "$_ejc_val" != "true" ]; then
    EJC_BUILD_WAS_SKIPPED=true
    return 0
  fi

  # --- Build cache: FORCE_REBUILD / CLEAN_BUILD / marker check ---
  _marker_file="$EJC_ELK_ROOT/.ejc-build-marker"

  ejc_resolve_var FORCE_REBUILD false
  _force_rebuild=$_ejc_val

  ejc_resolve_var CLEAN_BUILD false
  _clean_build=$_ejc_val

  if [ "$_force_rebuild" = "true" ]; then
    rm -f "$_marker_file"
    echo "ejc: FORCE_REBUILD=true; marker cleared"
  elif [ "$_clean_build" = "true" ]; then
    ejc_clean_all_targets
    echo "ejc: CLEAN_BUILD=true; targets and marker cleaned"
  else
    # Check if marker is valid
    _ejc_compute_build_key
    _current_key=$_ejc_val
    if [ -f "$_marker_file" ]; then
      _cached_key=$(cat "$_marker_file")
      if [ "$_cached_key" = "$_current_key" ]; then
        echo "ejc: build cache fresh (marker match); skipping Maven build"
        EJC_BUILD_WAS_SKIPPED=true
        return 0
      fi
    fi
  fi

  # --- Build ---
  ejc_resolve_var MVN_BIN mvn
  _mvn=$_ejc_val

  set -- "$_mvn" -f "$_pom"

  ejc_resolve_var_raw PREPARE_MODULES
  if [ -n "$_ejc_val" ]; then
    set -- "$@" -pl "$_ejc_val" -am
  fi

  ejc_resolve_var_raw MVN_LOCAL_REPO
  if [ -n "$_ejc_val" ]; then
    set -- "$@" "-Dmaven.repo.local=$_ejc_val"
  fi

  ejc_resolve_var_raw PREPARE_ARGS
  if [ -n "$_ejc_val" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $_ejc_val
  fi

  ejc_resolve_var_raw MVN_ARGS
  if [ -n "$_ejc_val" ]; then
    # shellcheck disable=SC2086
    set -- "$@" $_ejc_val
  fi

  if [ "$_clean_build" = "true" ]; then
    set -- "$@" clean install
  else
    set -- "$@" install
  fi
  ejc_run_cmd "$@"

  # --- Write marker on success (skip in dry-run mode) ---
  ejc_resolve_var DRY_RUN false
  if [ "$_ejc_val" != "true" ]; then
    _ejc_compute_build_key
    printf '%s' "$_ejc_val" > "$_marker_file"
    echo "ejc: build marker written: $_marker_file"
  fi
}

# ---------- DNS preflight ---------------------------------------------------

# _ejc_can_resolve_host HOSTNAME
#   Returns 0 if HOSTNAME can be resolved via any available tool.
_ejc_can_resolve_host() {
  _host=$1

  if command -v getent >/dev/null 2>&1; then
    if getent hosts "$_host" >/dev/null 2>&1; then
      return 0
    fi
  fi

  if command -v dscacheutil >/dev/null 2>&1; then
    if dscacheutil -q host -a name "$_host" 2>/dev/null | grep -q '^ip_address:'; then
      return 0
    fi
  fi

  if command -v dig >/dev/null 2>&1; then
    if [ -n "$(dig +short "$_host" 2>/dev/null | awk 'NF { print; exit }')" ]; then
      return 0
    fi
  fi

  if command -v nslookup >/dev/null 2>&1; then
    if nslookup "$_host" >/dev/null 2>&1; then
      return 0
    fi
  fi

  return 1
}

# ejc_dns_preflight
#   Checks that required hosts can be resolved.
#   Reads ${EJC_PREFIX}_DRY_RUN, ${EJC_PREFIX}_SKIP_DNS_CHECK,
#   ${EJC_PREFIX}_REQUIRED_HOSTS (comma-separated).
ejc_dns_preflight() {
  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  ejc_resolve_var SKIP_DNS_CHECK false
  [ "$_ejc_val" = "true" ] && return 0

  ejc_resolve_var REQUIRED_HOSTS "repo.eclipse.org,repo.maven.apache.org"
  _hosts=$_ejc_val

  [ -z "$_hosts" ] && return 0

  _unresolved=""
  _OLD_IFS=$IFS
  IFS=','
  # shellcheck disable=SC2086
  set -- $_hosts
  IFS=$_OLD_IFS

  for _h in "$@"; do
    _trimmed=$(printf '%s' "$_h" | awk '{ gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0); print }')
    [ -z "$_trimmed" ] && continue
    if ! _ejc_can_resolve_host "$_trimmed"; then
      if [ -n "$_unresolved" ]; then
        _unresolved="$_unresolved,$_trimmed"
      else
        _unresolved="$_trimmed"
      fi
    fi
  done

  if [ -n "$_unresolved" ]; then
    echo "ejc: dns preflight failed: unresolved hosts=$_unresolved" >&2
    echo "hint: fix DNS/network access, or set ${EJC_PREFIX}_SKIP_DNS_CHECK=true to bypass." >&2
    exit 1
  fi
}

# ---------- Java file injection / restoration -------------------------------

# Internal state for inject/restore tracking.
_ejc_injected_file_dest=
_ejc_injected_file_backup=
_ejc_injected_was_present=false
_ejc_injected_written=false

# ejc_inject_java_file SRC DEST
#   Copies SRC to DEST, backing up any existing file at DEST.
#   Skipped when ${EJC_PREFIX}_DRY_RUN=true.
ejc_inject_java_file() {
  _src=$1
  _dest=$2

  if [ ! -f "$_src" ]; then
    echo "ejc: missing java source: $_src" >&2
    exit 1
  fi

  ejc_resolve_var DRY_RUN false
  [ "$_ejc_val" = "true" ] && return 0

  _ejc_injected_file_dest=$_dest

  mkdir -p "$(dirname "$_dest")"
  if [ -f "$_dest" ]; then
    _ejc_injected_was_present=true
    _ejc_injected_file_backup=$(mktemp "${TMPDIR:-/tmp}/ejc-java-backup.XXXXXX")
    cp "$_dest" "$_ejc_injected_file_backup"
  fi
  cp "$_src" "$_dest"
  _ejc_injected_written=true
}

# ejc_restore_java_file
#   Restores or removes the file injected by ejc_inject_java_file.
#   Reads ${EJC_PREFIX}_BENCH_CLEANUP (default true).
ejc_restore_java_file() {
  ejc_resolve_var BENCH_CLEANUP true

  if [ "$_ejc_injected_written" = "true" ] && [ "$_ejc_val" = "true" ]; then
    if [ -n "$_ejc_injected_file_backup" ] && [ -f "$_ejc_injected_file_backup" ]; then
      cp "$_ejc_injected_file_backup" "$_ejc_injected_file_dest"
    elif [ "$_ejc_injected_was_present" = "false" ]; then
      rm -f "$_ejc_injected_file_dest"
    fi
  fi

  if [ -n "$_ejc_injected_file_backup" ] && [ -f "$_ejc_injected_file_backup" ]; then
    rm -f "$_ejc_injected_file_backup"
  fi

  # Reset state for potential re-use
  _ejc_injected_file_dest=
  _ejc_injected_file_backup=
  _ejc_injected_was_present=false
  _ejc_injected_written=false
}
