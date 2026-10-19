#!/bin/sh
# Publish the elk-rs npm packages: the six @openedges/elk-layout-* platform
# packages first, then the main `@openedges/elk-layout` package, skipping every package@version that is
# already on the registry (npm never allows republishing a version).
#
# Usage: sh scripts/publish_npm.sh [--dry-run] [--expect-version <version>]
#
#   --dry-run          `npm publish --dry-run`; missing build artifacts only warn
#   --expect-version   fail unless every package.json carries this version
#                      (CI passes the tag's base version, e.g. 0.11.1 for v0.11.1+rs.7)
#
# Environment:
#   NODE_AUTH_TOKEN   npm token (or log in with `npm login`); not needed for --dry-run
#   NPM_DIST_TAG      dist-tag to publish under (default: latest)
#
# Every package is validated (version, build artifact present) before the first
# publish so a bad main package cannot leave the platform packages published alone.
# Expects the platform `.node` binaries in plugins/org.eclipse.elk.js/npm/<triple>/
# and the WASM build in plugins/org.eclipse.elk.js/dist/wasm/ (see build.sh and
# .github/workflows/napi.yml). Run from the repository root.
set -eu

dry_run=false
expect_version=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=true ;;
    --expect-version) shift; expect_version="${1:-}"; [ -n "$expect_version" ] || { echo "publish_npm: --expect-version needs a value" >&2; exit 2; } ;;
    -h|--help) awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"; exit 0 ;;
    *) echo "publish_npm: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

[ -f Cargo.toml ] || { echo "publish_npm: run from the repository root" >&2; exit 1; }
js_dir="plugins/org.eclipse.elk.js"
dist_tag="${NPM_DIST_TAG:-latest}"

pkg_field() { # pkg_field <dir> <field>
  node -p "require('./$1/package.json').$2"
}

artifact_check() { # artifact_check <dir> <description> <glob>
  if ! ls $3 >/dev/null 2>&1; then
    if [ "$dry_run" = true ]; then
      echo "publish_npm: warning: $1 has no $2 (dry run continues)"
    else
      echo "publish_npm: $1 has no $2; refusing to publish an empty package" >&2
      exit 1
    fi
  fi
}

# Publish order: platform packages, then the main package that depends on them.
dirs=""
for dir in "$js_dir"/npm/*/ "$js_dir"; do
  dir="${dir%/}"
  [ -f "$dir/package.json" ] && dirs="$dirs $dir"
done

# Pass 1: validate every package before touching the registry.
for dir in $dirs; do
  name="$(pkg_field "$dir" name)"
  version="$(pkg_field "$dir" version)"
  if [ -n "$expect_version" ] && [ "$version" != "$expect_version" ]; then
    echo "publish_npm: $name has version $version, expected $expect_version" >&2
    exit 1
  fi
  if [ "$dir" = "$js_dir" ]; then
    artifact_check "$dir" "dist/wasm/*.wasm" "$dir/dist/wasm/*.wasm"
  else
    artifact_check "$dir" "*.node binary" "$dir/*.node"
  fi
done

# Pass 2: publish what is missing.
published=0
skipped=0
for dir in $dirs; do
  name="$(pkg_field "$dir" name)"
  version="$(pkg_field "$dir" version)"
  # `npm view` prints the version when it exists, nothing when the package exists
  # without that version, and fails (E404) when the package does not exist yet.
  if [ -n "$(npm view "$name@$version" version 2>/dev/null || true)" ]; then
    echo "publish_npm: skip $name@$version: already on the registry"
    skipped=$((skipped + 1))
    continue
  fi
  if [ "$dry_run" = true ]; then
    echo "publish_npm: dry run $name@$version (--tag $dist_tag)"
    (cd "$dir" && npm publish --access public --tag "$dist_tag" --dry-run)
  else
    echo "publish_npm: publishing $name@$version (--tag $dist_tag)"
    (cd "$dir" && npm publish --access public --tag "$dist_tag")
  fi
  published=$((published + 1))
done
echo "publish_npm: done (published=$published skipped=$skipped dry_run=$dry_run)"
