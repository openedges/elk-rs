# Connected-component compaction and numeric parity

Date: 2026-10-23. Follow-up to [layout improvements](layout_improvements.md),
commit `1d9f3afd`, on `fix/layout-improvements`.
Reference: ELK v0.12.0 (`external/elk` at `32331291`).

## Result and scope

Rust now uses rectilinear hulls and repeated one-dimensional compaction, including
external-edge extensions, placeholders, component spacing, margins, and directional
locks. This replaces the previous x-anchor alignment approximation. The existing
shared hull builder and compaction solver are reused; no parallel solver or new
public API was introduced. The feature remains controlled by
`org.eclipse.elk.layered.compaction.connectedComponents` (default `false`).

The original Java defect is corrected at the scanline event ordering boundary.
Regular input dimensions are preserved. Java's existing minimum-size rules for
synthetic external-port dummies and extension rectangles are retained.

The earlier conclusion that finite Rust output alone made the simplified
implementation preferable was incomplete. Normal inputs need the actual Java
algorithm, while the defective boundary condition needs a narrowly scoped fix.

## Root cause of ticket 213

The original Java scanline orders an interval's end before its start when their
coordinates are equal. A zero-height rectangle therefore receives its end event
before it has entered the active set, then remains active after its start. Stale
overlap constraints can form a cycle between component groups. The longest-path
solver does not visit those groups, leaving node positions at `-Infinity`.
Subsequent coordinate calculations propagate NaN.

This is a constraint-generation defect, not simply a bounding-box initialization
problem. The first differing trace processor is where the bad result becomes
observable, not necessarily where it originated.

The minimal reproduction uses only three rectangles. Each row is
`[group, x, y, width, height]`:

```json
[[3, 49, 59, 1, 0], [4, 41, 89, 20, 20], [4, 41, 109, 0, 30]]
```

At each scan coordinate the corrected order is:

1. End positive-height intervals.
2. Start and end each zero-height interval together, ordered by node ID.
3. Start positive-height intervals.

Ordering among nondegenerate events is unchanged. Zero-size inputs can change
layout even when the original Java output was finite: in `zero_size_components`,
node `n5.y` changes from 52 to 12 in both corrected Java and Rust. This is an
intentional consequence of removing stale zero-height constraints. The Rust regression checks all
four compaction directions and equal-coordinate event ties. The original Java
minimal reproduction reports `nonfinite`; the corrected reference reports `finite`.

## Reference provenance

[references.json](component_compaction/references.json) identifies the Java source
used for every checked-in expected output. There are 16 reference fixtures:
14 use unmodified Java v0.12.0; `ticket_213_zero_size` and
`zero_size_components` use v0.12.0 plus the explicit
[scanline patch](component_compaction/java/zero-extent-scanline.patch).
The external-port fixtures are nested inside a parent node to exercise the normal
Java compound-node path. Cases cover four layout directions, four external-port
sides, disconnected and port-only components, labels, spacing, self-loops,
zero-size nodes, and disabled compaction.

The corrected reference is compiled into a separate directory. It does not modify
`external/elk`, replace the official full-corpus baseline, or claim that upstream
has accepted the fix. Expected JSON comes from Java, not from the Rust candidate.
The ticket fixtures derive from `external/elk-models/tickets/layered/213_componentsCompaction.elkt`;
the positive variant changes explicit zero widths/heights to 20. Other inputs are
small purpose-built fixtures. Original upstream attribution is retained in the
ported modules and Java patch source.

## Initial compaction validation (before the phase-trace correction)

| Check | Result |
|---|---|
| Workspace build / Clippy with `-D warnings` | Pass, zero warnings |
| Workspace tests | 1,069 passed, 0 failed, 2 ignored |
| WASM target check / rustfmt of added or updated Rust modules | Pass |
| New Rust tests | 2 scanline tests + 17 JSON integration tests |
| Python numeric comparison tests | 4 passed, including NaN/±Infinity in both directions |
| Java golden fixtures | 16/16 match (14 original + 2 corrected), finite output |
| All 1,998 Rust final outputs vs previous commit | 1,997 unchanged; only ticket 213 changed |
| Original Java full model comparison | 1,988/1,989 match; ticket 213 differs; 9 Java non-ok cases skipped |
| Corrected Java ticket 213 | Final layout matches; all 229 trace steps match |
| Original Java full phase comparison, corrected numeric checks | 1,827/1,997 match; 170 flagged; 1 Rust-only model |
| Previous commit's full phase traces under the same numeric checks | Same 1,827 match / 170 flagged; only ticket 213's details changed |

The table above preserves the initial compaction-stage measurements. The later
[full phase-trace correction](phase_trace_audit.md) resolves all 169 resize cases,
aligns ticket 588 as an expected error, and removes the Java-zero exception and
Rust snapshot adjustments. Its strict gate accounts for all 1,998 inputs:
1,997 matching layouts, one matching expected error, and 196,407 matching raw
processor states. The external-port single-component path now uses group
placement even when there is only one split component.

The current totals are in [phase_trace_validation.json](phase_trace_validation.json).
[validation-summary.json](component_compaction/validation-summary.json) records
only the earlier compaction-stage results. No new benchmark or performance
guarantee is claimed for this opt-in algorithm.

## Reproduce

Rust and Python regression tests:

```sh
cargo test -p org-eclipse-elk-alg-common scanline_constraint_calculator
cargo test -p org-eclipse-elk-graph-json --test component_compaction
python3 -B -m unittest discover -s scripts/tests -p 'test_*.py'
```

To independently verify the Java minimal reproduction and all expected fixtures,
provide a classpath containing the unmodified ELK v0.12.0 compiled modules and
runtime dependencies (Gson, Guava, EMF, etc.). `external/elk` must be the pinned
source checkout. The output directory must be empty:

```sh
python3 -B tests/component_compaction/java/check_reference.py \
  --elk-source external/elk \
  --classpath "$ELK_REFERENCE_CLASSPATH" \
  --output /tmp/elk-component-reference
```

The checker compiles the original probes and a separate corrected class, applies
the Java patch only to a copied source file, verifies the three-rectangle failure
and fix, and compares all generated layouts with the checked-in fixtures.
For the full corpus, use the standard procedures in `TESTING.md`; keep original
Java and corrected Java evidence separate.
