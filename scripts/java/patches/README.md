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

`scripts/run_java_model_parity_export.sh` applies all `*.patch` files from
this directory (in lexicographic order) to the isolation worktree immediately
after creation and before the Java build. The isolation worktree is deleted
during cleanup, so patches are automatically reverted.

Set `JAVA_PARITY_APPLY_PATCHES=false` to skip patch application.

## Patch inventory

| Patch | Issue |
|-------|-------|
| `0001-deterministic-opposing-self-loop-routing.patch` | `SelfHyperLoop.computePortsPerSide()` uses `ArrayListMultimap` (HashMap-backed) whose `keySet()` iteration order varies across JVM runs. Opposing self-loop tie-breaks depend on this order, causing ~80 models to flip. Fix: use `MultimapBuilder.enumKeys()` for deterministic enum-ordinal iteration. |

## Adding a new patch

1. Create a standard `git diff` or `git format-patch` output.
2. Name it with a sequential prefix: `0002-short-description.patch`.
3. Test: run `scripts/run_java_model_parity_export.sh` and verify the patch
   applies cleanly and the build succeeds.
4. Add a row to the inventory table above.
