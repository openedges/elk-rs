# Full phase-trace validation

Date: 2026-10-23. Branch: `fix/layout-improvements`.
Base commit: `1d9f3afd4bcc402ea3e380eaa01a4561a36456ce`.
Java reference: ELK v0.12.0 (`32331291`) with the three explicit
[reference patches](../scripts/java/patches/README.md).

## Result

**The strict full-corpus gate passes: 1,998 inputs accounted for, 1,997 matching
successful layouts, one matching expected error, and 196,407 matching processor
states.** There are no missing models, unexpected errors, trace differences, or
nonfinite values in successful layouts and snapshots.

This is a corrected Java reference, not an assertion that unmodified upstream
Java has no defects. The patches are compiled in a separate source/class overlay;
`external/elk` remains unchanged. No upstream acceptance or release is implied.
The checked-in [validation summary](phase_trace_validation.json) records counts,
source revisions, patch hashes, and hashes of the local detailed reports.

## Causes and corrections

| Finding | Cause | Implemented correction |
|---|---|---|
| Nonfinite port coordinates in 169 inputs | Both resize helpers divide by an old node dimension of zero | Use ratio 1 for a collapsed axis; preserve ordinary scaling and border movement |
| Ticket 213 nonfinite component geometry | A zero-height scanline interval ends before it starts, leaving a stale active interval and cyclic constraints | Process each zero-height start/end together; use the actual hull/1D component algorithm |
| Raw external-component width differed by 20 | Rust's single-component shortcut bypassed Java's external-port group placement | Only take the shortcut for a non-external component or the unsplit source graph |
| Ticket 588 appeared as Rust-only success | Rust silently discarded an unsupported cross-hierarchy edge | Report `UnsupportedGraphException`; the JSON API returns `Err` |
| Later traces disappeared after an exception | Shared nesting counters were not restored on unwind | Thread-local counters with an RAII nesting guard |
| Old gate passed incomplete or invalid evidence | Broad Java-zero exception, geometry rewriting, missing coverage and fields | Raw snapshots, strict schema/finite checks, structural endpoint mapping, exact manifest accounting |

### Zero-size resize contract

The analysis found **400 invalid resize calls across 169 inputs**: core
`ElkUtil` handled 399 calls in 168 models, and layered `LGraphUtil` handled one
call in ticket 316. These inputs represent 59 model stems, including repeated
input formats. The original exports contained 18,053 observations of nonfinite
port coordinates. They were arithmetic defects, not an intentional unassigned
coordinate marker. Later port placement overwrote them, hiding the problem from
final-output comparisons.

Both helpers now use `oldSize == 0 ? 1 : newSize / oldSize` independently per
axis. This preserves the existing tangent coordinate on a collapsed axis;
EAST/SOUTH ports still move with the appropriate border delta. Fixed positions
and `movePorts=false` retain their original meaning. The layered helper retains
its Java-compatible float conversion for ordinary scaling.

Both helpers' label placement infers a relative region from both old
dimensions. If either old dimension is zero, that region is undefined, so labels
retain their existing coordinates for this resize. Ordinary two-dimensional
label repositioning is unchanged. Tests cover one/both collapsed axes,
zero-to-zero, every port side, fixed positions, disabled movement, repeated
resize, labels, and the returned ratio.

### Faithful intermediate states

The Rust recorder no longer calls `apply_trace_compaction`. Its coordinate
rewrites, step-number conditions, and 24-pixel conditions were removed. The
external-component branch was fixed in the engine itself after raw exports
exposed the difference; it is covered by a focused regression test.

Both exporters record node geometry, margins and padding; port geometry,
anchors and margins; labels; edge endpoint identities and bend points; layer
membership; and graph size, inner size and offset. Snapshot-local unique IDs
replace mutable processor scratch IDs. `externalNodes` describes endpoints in
nested graphs that compound postprocessing can reference.

Rust trace nesting is thread-local and restored on early return or unwind.
Snapshot write failures propagate. The Java exporter does not retry a failed
recursive layout through a different provider, and errors produce a nonzero
exit status. The corpus gate decides whether each error was explicitly expected.

## Comparison contract

The full workflow sets root algorithm `org.eclipse.elk.layered`, random seed 1,
and recursive execution on both sides, while preserving nested algorithm
options. It uses identical input JSON and checks unchanged input hashes.

- Final layouts use absolute numeric tolerance `1e-6`; processor states use
  `0.001`. NaN and infinity fail even when both sides contain them. Zero-valued
  Java coordinates are compared normally.
- Snapshot IDs and container storage order are language-specific. The comparator
  sorts nodes within a layer and maps ports using geometry plus incident-node
  identities, then compares actual endpoint connectivity and layer membership.
  Signed zero is normalized only in identity sorting. Geometry is not repaired.
- Processor order, component order, snapshot fields, step continuity, and the
  Java index/file inventory are checked. Empty, missing, duplicate or malformed
  results fail. The independent input manifest, Java manifest and Rust manifest
  must describe exactly the same models, including specified errors.
- Ticket 588 is listed in [phase_trace_expected_errors.json](phase_trace_expected_errors.json).
  Both sides must return `org.eclipse.elk.core.UnsupportedGraphException`.
  A timeout, a different exception, an unexpected success, or an unknown expected
  error fails. This input stays in the denominator; it is not a skipped success.

This is equality of the exported geometry and topology under the documented
representation mapping and tolerances, not byte equality or an assertion that
every internal implementation detail is identical.

## Validation

| Check | Result |
|---|---|
| Complete manifest coverage | 1,998 / 1,998 |
| Corrected Java vs Rust final layouts | 1,997 / 1,997 |
| Explicit expected-error cases | 1 / 1, matching exception |
| Raw processor snapshots | 196,407 / 196,407 |
| Failed models / successful nonfinite geometry | 0 / 0 |
| Workspace tests | 1,081 passed, 0 failed, 2 ignored |
| Python comparator/accounting tests | 21 passed |
| Workspace build / Clippy `-D warnings` | Pass, zero warnings |
| `wasm32-unknown-unknown` check | Pass |
| Fresh reference/build/export/verification workflow smoke test | 3 inputs: 2 successful, 1 expected error; 135 states match |
| Ordinary-algorithm final-output regression | All 1,997 successful outputs unchanged from the completed compaction implementation |

New Rust coverage in this phase consists of eight resize/error integration tests,
one external-endpoint snapshot test, two trace-scope tests, and one
external-component placement test. Existing snapshot tests now assert actual
engine values rather than recorder-adjusted values. Python regressions exercise
nonfinite values, zero comparisons, geometry/topology fields, schema validity,
missing/empty/duplicate steps and models, and expected-error accounting.

The ordinary-algorithm regression also retained the unmodified Java comparison:
1,988 matches, ticket 213 differs, and eight historical Java layouts are absent.
Ticket 588 is now an explicit error. These historical original-Java artifacts are
separate from the complete corrected-reference run; missing layouts do not pass
the new gate. Original 1,827/1,997 phase matches with 170 flagged inputs and one
Rust-only input describe the pre-fix audit only. Earlier 1,997/1,997 claims from
the permissive comparator must not be used as raw-state equivalence evidence.

Repository-wide `cargo fmt --all -- --check` encounters existing formatting drift
outside this task. Added Rust modules are formatted, modified sections were
reviewed, and `git diff --check` passes; no repository-wide reformat is included.
This work does not claim a new performance benchmark or a package release.

## Reproduce

Prerequisites: Python 3.10+, the pinned Rust toolchain, a JDK, and compiled Java
ELK v0.12.0 modules plus runtime dependencies (Gson, Guava, EMF, etc.). Include
`test/org.eclipse.elk.alg.test/target/classes` in the classpath for the exporter's
existing standalone entry point. Build the pinned Java checkout using the
procedures in [TESTING.md](../TESTING.md).

Use a complete input TSV from the Java model export. Its first two columns are
`model_rel_path` and `input_json`; the standard additional columns are
`java_layout_json`, `java_status`, and `java_error`. Input JSON files must exist
for error cases too. The workflow independently enumerates `.elkt`, `.elkg`, and
`.json` under the model root and rejects incomplete manifests. The old status
columns are not reused as outcomes: both engines execute the inputs afresh.

```sh
export JAVA_TRACE_CLASSPATH='<pinned ELK module classes and dependency JARs>'
export PHASE_INPUT_MANIFEST='/absolute/path/to/complete-input.tsv'
sh scripts/run_full_trace_parity.sh external/elk-models /tmp/elk-full-phase
```

The output directory must not already exist. The workflow copies the four
patched Java source files, compiles the overlay, builds Rust in the normal
workspace target, exports both engines, and runs `verify_phase_corpus.py`.
`report.json` and `verification.log` must show `status: pass`, 1,998 expected inputs,
1,997 successful layouts, one expected error and zero failed models.
`provenance.json` records source revisions, tracked-diff and new-file hashes,
reference patch hashes and every input hash. Raw traces and layouts are temporary
artifacts; keep only compact reports in the repository.

Focused regressions:

```sh
cargo test -p org-eclipse-elk-graph-json --test resize_geometry
cargo test -p org-eclipse-elk-alg-layered --test general
cargo test -p org-eclipse-elk-alg-layered single_external_component_uses_group_placement
python3 -B -m unittest discover -s scripts/tests -p 'test_*.py'
```

All completion criteria from the root-cause analysis are satisfied: actual
arithmetic and component placement are corrected, snapshots are faithful,
coverage and numerical checks are strict, errors are aligned, and the full
corrected reference and supported-target checks pass. The runtime changes were
committed as `4b9a1f56`; [0.12.1 release validation](release_0_12_1.md) records
subsequent package checks and performance measurements. External submodules are unchanged.
