# Java ELK Parity Patches

Patches in this directory are applied to the **isolation worktree/copy** of
`external/elk` during Java model-parity export runs. They are never applied to
the original `external/elk` submodule.

## Why patches?

Java ELK has a few non-deterministic code paths that produce different layout
results across JVM invocations. Since the parity test compares Java output
against Rust output, non-deterministic Java results cause spurious drift
(~80 models flipping between runs).

These patches make the affected Java code deterministic so that parity
comparisons are stable and reproducible.

## How patches are applied

`scripts/java_model_parity_trace.sh` and `scripts/java_model_phase_step_trace.sh`
apply all `*.patch` files from this directory (in lexicographic order) to the
isolation worktree immediately after creation and before the Java build.

- **Ephemeral isolation** (default): The isolation worktree is deleted during
  cleanup, so patches are automatically reverted.
- **Persistent isolation** (`JAVA_PARITY_ISOLATION_DIR` / `JAVA_TRACE_ISOLATION_DIR`):
  Patches are applied once and preserved across runs. The build cache key includes
  a checksum of all patch files, so if patches change the cached isolation is
  automatically invalidated and recreated.

Set `JAVA_PARITY_APPLY_PATCHES=false` (or `JAVA_TRACE_APPLY_PATCHES=false` for
the phase trace script) to skip patch application.

## Patch inventory

| Patch | Issue |
|-------|-------|
| `0001-deterministic-opposing-self-loop-routing.patch` | `SelfHyperLoop.computePortsPerSide()` uses `ArrayListMultimap` (HashMap-backed) whose `keySet()` iteration order varies across JVM runs. Opposing self-loop tie-breaks depend on this order, causing ~80 models to flip. Fix: use `MultimapBuilder.enumKeys()` for deterministic enum-ordinal iteration. |

## Adding a new patch

1. Create a standard `git diff` or `git format-patch` output.
2. Name it with a sequential prefix: `0002-short-description.patch`.
3. Test: run `scripts/java_model_parity_trace.sh` and verify the patch
   applies cleanly and the build succeeds.
4. Add a row to the inventory table above.

## Troubleshooting: patches apply but have no effect

**Symptom**: The export script reports `VERIFIED patch content in ...` but
the Java test output still shows unpatched behavior.

**Root cause**: Stale ELK SNAPSHOT JARs in the local Maven cache
(`~/.m2/repository/org/eclipse/elk/*/X.Y.Z-SNAPSHOT/`). Tycho/OSGi resolves
bundles by highest version number, so a leftover higher-versioned SNAPSHOT
(e.g. `0.12.0-SNAPSHOT`) silently overrides the freshly built patched JARs
at test runtime.

**Prevention**: The export script automatically purges all ELK SNAPSHOT
directories from the Maven cache before building. If you use a custom
`JAVA_PARITY_MVN_LOCAL_REPO`, the purge targets that directory instead.

**Manual fix** (if the guard is bypassed or insufficient):

```sh
find ~/.m2/repository/org/eclipse/elk -name '*-SNAPSHOT' -type d -exec rm -rf {} +
```

Then re-run the full parity export (without `MODEL_PARITY_SKIP_JAVA_EXPORT`).
