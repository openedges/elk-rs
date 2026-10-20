# Custom ELK 0.12.0 integration — 2026-10-20

`custom/0.12.0` starts from `custom/0.11.0` at `5473eb06351eb09a33a76279623c4ad9e5cfe034`
and merges `releases/0.12.0` at `10661c0e65186e1b2b07cfdab9ccd444e467c337`.
Cargo/npm manifests are `0.12.0-ext.1`. The previous custom branch and existing
tags are preserved. After integration, annotated tag
[`v0.12.0-ext.1`](https://github.com/openedges/elk-rs/tree/v0.12.0-ext.1) was pushed
at `371e186d` on user request. It triggers build-only run
[39372006098](https://github.com/openedges/elk-rs/actions/runs/39372006098);
no GitHub Release or registry publication is requested.

The Java reference is `v0.12.0` (`323312916048544e4e594eab3303cdb0d01e2929`);
elkjs is `0.12.0` (`ff5771d7165445c42c408bb8a090c8035272218c`). The Java exports
generated for the stable 0.12.0 upgrade were reused with these unchanged pins.
All Rust/NAPI/WASM outputs below were regenerated from the merged custom tree.

## Changes and review

- Retained the existing `ignoreEdgeInLayer`, FIRST/LAST_SEPARATE routing,
  external-port layer handling, edgeless port fixes, and elk-live changes.
- Included the stable port's `NodePlacementRepeater`, nullable recompute option,
  exact unzip boundary fix, JS entry-point/WASM packaging changes, license
  notices, and OIDC-only authentication.
- Resolved version/document conflicts and retained both appended metadata tests.
  Independent review confirmed that the custom runtime source delta against
  the release still matches the previous custom delta.
- Added three cross-feature tests: five placers × ignored/normal edges with
  original E/W port anchors; exact split=2 unzipping; and all 18 extension
  fixtures × five recompute placers, including compound graphs.
- Closed the manual-publish bypass for extension versions. Preflight rejects
  prerelease `workflow_dispatch` requests with either real-publish input true.
  Default manual builds/dry runs and pushed extension builds are preserved.

## Validation

| Check | Result |
|---|---|
| Workspace build / Clippy | 0 errors, 0 warnings |
| Rust workspace tests | 1,058 passed, 0 failed, 2 ignored |
| New cross-feature tests | 3 passed; includes 90 fixture/placer combinations |
| Existing extension fixtures | 18/18 unchanged |
| Full model parity | 1,987/1,989 matches (99.90%); 2 known differences, 9 skipped, 0 errors/timeouts; 1,998 inputs |
| Full phase-step trace | 1,995/1,997 matches (99.90%); 2 known differences, Java-only 0, Rust-only 1 |
| New upstream feature fixtures | 8/8 model and 8/8 trace matches |
| Vitest | 49/49 passed |
| JS parity against Java | NAPI 550/550 and web WASM 550/550; each 530 PASS + 20 ELKJS_DRIFT, no missing Java or failures |
| New feature JS parity | NAPI 8/8 and web WASM 8/8 |
| Extension JS fixture outputs | NAPI, web WASM, Node CJS WASM each 18/18 unchanged |
| Static checks | 12 reports: 11 status `ok`; option-support difference is the known custom option |
| Workflow | actionlint/ShellCheck passed; 9 preflight cases passed |
| Version consistency | Cargo workspace and all npm manifests/locks: `0.12.0-ext.1` |
| Linux CI | [39369695146](https://github.com/openedges/elk-rs/actions/runs/39369695146): passed on merge commit `d8363613`; 1,058 passed, 0 failed, 2 ignored; 0 Rust warnings |
| elk-live | Fresh custom WASM copied and byte-checked; production build and 22 ELKT/ELKEX parser unit tests passed |
| Release rehearsal | [39369716186](https://github.com/openedges/elk-rs/actions/runs/39369716186): all six NAPI platforms, both WASM targets, npm 7-package and crates.io 18-crate dry runs passed |

Model differences are `tickets/layered/213_componentsCompaction.elkt` (existing
Java NaN/compaction issue) and `tests/layered/layer_constraints/emptyLayerFirst.elkt`
(existing extension behavior). Trace differences first occur at
`emptyLayerFirst.elkt` step 5 `LongEdgeSplitter` and `emptyLayerLast.elkt` step 6
`PortSideProcessor`, as on the previous custom line. Rust-only trace is
`588_hierarchicalLayoutNPE.elkt`, whose Java trace cannot be exported.
There are no new corpus differences.

The explicit-null feature fixture remains outside Java parity because the Java
importer throws on `JsonNull`; Rust's disabled-option behavior is covered by
the retained regression tests. Static option-support counts are Java153/Rust154;
option-default has one informational extra pair:
`org.eclipse.elk.layered.layering.ignoreEdgeInLayer`.

Extension fixture comparison removes internal `$` keys and rounds numbers to
four decimal places, matching the existing Rust fixture tests.

## Reproduction and environment limits

Build and test with the pinned Rust 1.97.1 / Node 24 toolchains:

```sh
cargo build --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p org-eclipse-elk-graph-json custom_recompute_test
python3 scripts/bump_version.py --check
```

Use the model and trace commands in `TESTING.md` §2.B with a Java **0.12.0**
baseline. Run the JS commands in §3.4 with an explicit freshly built backend.
Raw outputs for this run are under `/tmp/elk-custom-012-validation/`; the
portable summary is [validation.json](upgrade_custom_0_12_0/validation.json).

Local NAPI was built with `npm exec --package=@napi-rs/cli -- napi build --release
--platform --output-dir ../org.eclipse.elk.js/dist`; web and Node targets were
built by `wasm-pack`. The machine lacked a globally installed `napi` command.
With intentionally unpublished extension optional packages, npm 11.17 `npm ci`
fails on missing optional lock entries. `npm install --ignore-scripts` installs
the development dependencies; the tests use local artifacts. Node's direct
import of browser ESM WASM emits the existing module-type detection notice.

Performance was not remeasured for this custom integration. The stable upgrade
benchmark in [upgrade_0_12_0.md](upgrade_0_12_0.md) is a separate measurement.
Remote CI and release rehearsal ran on merge commit `d8363613`; subsequent
changes only record these results. The workflow was dispatched with both
publish inputs false. Seven artifacts (six NAPI binaries and one WASM bundle)
were uploaded; npm/crates.io publish and GitHub Release jobs were all skipped.
