# elk-rs Versioning and Operations Policy

## Context

elk-rs is a Rust port of Java ELK (Eclipse Layout Kernel) and also provides an elkjs-compatible npm package. Current state:

- **ELK Java** (`external/elk`): pinned to `v0.12.0`, commit `323312916048544e4e594eab3303cdb0d01e2929`
- **elkjs** (`external/elkjs`): pinned to `0.12.0`, commit `ff5771d7165445c42c408bb8a090c8035272218c`
- **elk-models** (`external/elk-models`): pinned to the parity corpus commit
- **Working tree**: `custom/0.12.0`, all Rust crates and npm packages `0.12.0-ext.2`; the JS test dependency is exact `elkjs@0.12.0`. Merge commit `3029a99b` incorporates stable `0.12.1` (`eb586b6d`) into the previous custom tip (`7d8e9791`). Annotated tag `v0.12.0-ext.2` points to that merge and is pushed to `github`, `origin`, and `gitlab`; `v0.12.0-ext.1` is preserved. These existing versions and tags retain their historical names.
- **Previous published package version**: `0.12.0`; release workflow 39357332958 completed publication on 2026-10-20
- **Latest stable release**: `0.12.1` (`v0.12.1`), an elk-rs patch on the existing `releases/0.12.0` branch; Java and elkjs pins remain `0.12.0`. See [release validation](tests/release_0_12_1.md).
- **Initial release tag**: [`v0.12.0`](https://github.com/openedges/elk-rs/releases/tag/v0.12.0), pointing to the original release commit `22fbee45`. The mistaken `v0.12.0+rs.7` tag was removed and the existing Release renamed, preserving all seven assets. The old `+rs.N` convention is retained as history; new release preparation follows the transition policy below.
- **Verification**: current custom results are recorded in [the ext.2 integration report](tests/update_custom_0_12_0_ext_2.md). The [stable 0.12.1 report](tests/release_0_12_1.md) and [previous custom report](tests/upgrade_custom_0_12_0.md) are separate historical validations.
- **Node.js runtime**: the 0.12.0 npm package requires Node.js 18+ (minimum 18.0.0 verified); development and release builds use pinned Node.js 24. Node.js 16 is unsupported because the standard WASM artifact requires reference types unavailable there without experimental flags.
- **License**: `EPL-2.0 OR GPL-3.0-or-later`, with the upstream conditional secondary-license notice preserved in `LICENSE` and `NOTICE`


This policy distinguishes the Java compatibility reference, the elk-rs package version, and build identity. The transition approved on 2026-10-23 applies to the next version after the existing `v0.12.0-ext.2` tag. It does not rename or retag an existing release.

---

## 1. Version Management

### Effective Date and Preserved Releases

The user approved this policy on **2026-10-23**, after `v0.12.0-ext.2` was
created. Apply it to the **next new version**. Current manifests stay at
`0.12.0-ext.2`; this documentation update does not prepare or tag a new package.

Preserve existing tags, their target commits, published packages, and historical
validation reports, including `v0.12.1`, `v0.12.0-ext.1`, `v0.12.0-ext.2`, and
all existing `+rs.N` tags. Do not move, delete, rename, or create a replacement
alias for `v0.12.0-ext.2` as part of this transition.

### Separate Version Meanings

| Item | Meaning | Current state / next-version rule |
|------|---------|-----------------------------------|
| Java ELK reference | Compatibility and parity baseline, recorded by tag and commit | Java `0.12.0`; independent of elk-rs patch increments |
| Stable package version | The elk-rs release identifier shared by Cargo and npm | `0.12.1`; a subsequent published patch would be `0.12.2` |
| Stable tag | Exact stable package version prefixed with `v` | `v0.12.1`, then `v0.12.2` when that version is prepared |
| New custom version | Included stable elk-rs package version plus an extension counter | `{STABLE_VERSION}-ext.{N}` |
| New custom tag | Exact custom package version prefixed with `v` | `v{STABLE_VERSION}-ext.{N}` |
| Intermediate build identity | Commit SHA and CI run ID / attempt | No new `+rs.N` tag for routine builds |

The existing major/minor family convention remains in use, but the elk-rs patch
number is not a Java patch identifier. Record the actual Java and elkjs pins
separately in the release notes. All workspace crates, npm packages, lockfiles,
and exact internal dependency pins must agree on the package version.

### Stable Releases and Intermediate Builds

- A new published fix increments the elk-rs package patch version and uses a
  bare annotated tag, for example `0.12.2` and `v0.12.2`.
- Use `scripts/bump_version.py` and the release validation checklist before
  tagging. A tag alone does not change the version in the manifests.
- Identify intermediate builds by their source commit and CI run ID. New
  releases do not introduce `+rs.N` tags. Preserve the existing ones as history.
- Rebuilds use the existing source ref and a new CI run/attempt; never move a
  released tag or replace the contents of a published package version.

### Extension Releases: Stable-Version Base

For new extension versions, the prefix is the **included stable elk-rs version**,
not the Java reference version. `N` starts at 1 for each stable base and increases
for subsequent custom versions on that same stable base.

| Next action | Package version / tag | Java reference |
|-------------|-----------------------|----------------|
| Next custom version still incorporates stable `0.12.1` | `0.12.1-ext.1` / `v0.12.1-ext.1` | `0.12.0` |
| Another custom revision on stable `0.12.1` | `0.12.1-ext.2` / `v0.12.1-ext.2` | `0.12.0` |
| Integrate a future stable `0.12.2` for the first time | `0.12.2-ext.1` / `v0.12.2-ext.1` | Record the actual pin; a Rust patch need not change it |

These are prospective examples, not tags or package versions created by this
policy update. If the next custom release already includes stable `0.12.2`,
start at `0.12.2-ext.1`; do not issue `0.12.1-ext.1` merely to fill the sequence.
The old `ext.2` counter from `0.12.0-ext.2` does not carry into the new stable base.

Before each custom release:

1. Merge the chosen stable release into the custom integration branch.
2. Record its stable package version, tag, and commit separately from Java/elkjs
   reference tags and commits.
3. Verify inherited extension fixtures, model parity, and raw phase parity;
   explicitly retain any known custom differences in the validation report.
4. Set Cargo/npm to `{STABLE_VERSION}-ext.{N}` and create the matching annotated tag.

Extension versions remain **build-only**: no npm/crates.io publication and no
GitHub Release page; the workflow retains build artifacts. The `-ext.N` suffix
is a SemVer prerelease identifier, so `0.12.1-ext.1 < 0.12.1`, even though the
custom build includes extra features. It does not represent a stable upgrade.
A future registry distribution of custom packages requires a separate package /
publication policy decision. See [SemVer](https://semver.org/#spec-item-9).

Existing integration branch names remain `releases/0.12.0` and `custom/0.12.0`.
A stable patch increment does not rename a branch or require `custom/0.12.1`.
For a new Java baseline, preserve custom history by creating the next integration
branch from the previous custom line and merging the corresponding stable line;
choose the custom package prefix from the stable version actually incorporated.

### Legacy Tags and Current Automation

Before this transition, custom names used `{JAVA_ELK_VERSION}-ext.N`, which is
why the preserved `v0.12.0-ext.2` includes stable `0.12.1`. Stable source revisions
also used `v<package-version>+rs.N` without changing the package version.
These old naming rules do not govern the next version.

The workflow is unchanged by this documentation decision:

- It strips the leading `v` and any `+...` suffix, then checks the resulting
  version against all manifests. New stable and custom names already fit this check.
- Pushed tags with a prerelease-free package version enable registry publication;
  publish scripts skip package/version pairs already present. Consequently, a
  legacy `+rs.N` tag is **not guaranteed to be build-only**: it may publish missing
  packages and create a GitHub Release after the required jobs succeed.
- Prerelease manifests, including `-ext.N`, skip publication and the GitHub
  Release job. Manual real-publish requests for these versions are rejected.
- The naming rules, stable-base correspondence, and intended branch are reviewed
  policy; preflight does not enforce all of them. Legacy `+rs.N` acceptance remains
  for compatibility and does not authorize new tags under the retired convention.

### CHANGELOG Rules

- Keep one entry per stable or custom version and record the included stable base
  and Java reference separately for custom releases.
- Keep historical `+rs.N` groups intact. Do not add new groups for future releases;
  unreleased changes belong under `Unreleased` until the next package is prepared.
- Use `Added`, `Fixed`, `Changed`, `ELK Porting`, and `Known Issues` as appropriate.
- Summarize user-visible changes and validation limits; keep detailed work logs
  in `HISTORY.md`.

### Version Sync Files

The values below describe the existing tagged versions, not a version bump made
by the policy transition. Apply the new naming rule when preparing the next version.

`python3 scripts/bump_version.py <version>` edits the manifest and lockfile rows below;
`--check` verifies their package versions agree. Update `CHANGELOG.md` manually.
The script does not change the `elkjs` development dependency: pin that explicitly
to the target upstream version and regenerate the npm lockfile.

| File | Version Field | Stable | Extension |
|------|---------------|--------|-----------|
| `Cargo.toml` (workspace root) | `[workspace.package] version` and the `version = "=X.Y.Z"` pins in `[workspace.dependencies]` | `"0.12.1"` | `"0.12.0-ext.2"` |
| `Cargo.lock` | Workspace member entries (`cargo update --workspace`) | `"0.12.1"` | `"0.12.0-ext.2"` |
| `plugins/org.eclipse.elk.js/package.json`, `package-lock.json` | `"version"`, the six `@openedges/elk-layout-*` optional dependencies, lockfile root entries | `"0.12.1"` | `"0.12.0-ext.2"` |
| `plugins/org.eclipse.elk.js/npm/*/package.json` | Six platform package versions | `"0.12.1"` | `"0.12.0-ext.2"` |
| `plugins/org.eclipse.elk.napi/package.json` | NAPI build package version (private) | `"0.12.1"` | `"0.12.0-ext.2"` |
| `packages/elk-live/package.json`, `package-lock.json` | Private application version | `"0.12.1"` | `"0.12.0-ext.2"` |
| `CHANGELOG.md` | Change history per published version | `## [0.12.1]` | `## [0.12.0-ext.2]` |

### Cargo Workspace Version Unification

Root `Cargo.toml` uses `workspace.package.version` so all crates inherit from a single
source, and `[workspace.dependencies]` carries the intra-workspace dependencies with the
`version` that `cargo publish` requires:

```toml
# Cargo.toml (root)
[workspace.package]
version = "0.12.0"
license = "EPL-2.0 OR GPL-3.0-or-later"
edition = "2021"
repository = "https://github.com/openedges/elk-rs"

[workspace.dependencies]
org-eclipse-elk-graph = { path = "plugins/org.eclipse.elk.graph", version = "=0.12.0" }

# Each crate's Cargo.toml
[package]
name = "org-eclipse-elk-core"
version.workspace = true
description = "..."              # required by crates.io
repository.workspace = true

[dependencies]
org-eclipse-elk-graph = { workspace = true }

[dev-dependencies]
org-eclipse-elk-alg-layered = { path = "../org.eclipse.elk.alg.layered" }   # path only
```

- The pins are exact (`=0.12.0`) because the crates are released together and share
  internal APIs; a consumer can never mix two elk-rs versions.
- `[dev-dependencies]` stay path-only on purpose. Cargo drops path-only
  dev-dependencies from the published manifest, which is what lets the test-only cycles
  (`core <-> alg-*`, `alg-common <-> alg-*`) publish.
- Each Cargo package includes a copy of root `LICENSE`, containing the EPL text and
  the applicable project notices, so registry archives retain the conditional secondary
  license notice and attribution. Keep these copies synchronized when licensing changes.
- `org-eclipse-elk-napi` and `org-eclipse-elk-wasm` are `publish = false`: they are
  cdylib bindings shipped through npm, not crates.io.

---

## 2. Submodule Pinning Policy

### Submodule Commit Pinning Rules

| Submodule | Pin Target | Rationale |
|-----------|------------|-----------|
| `external/elk` | Release tag (currently `v0.12.0`) | Parity reference point, reproducibility |
| `external/elkjs` | Release tag (currently `0.12.0`) | JS parity reference point |
| `external/elk-models` | Parity-verified commit | Model corpus for parity testing |

**Rules:**
- Submodules must always be pinned to a release tag (or a verified commit for elk-models)
- Submodule changes must include `chore: pin external/elk to v0.X.Y` in the commit message
- Never leave submodules pointing at SNAPSHOT or development commits

---

## 3. Branch and Tag Policy

### Branch Strategy

```
main                        <- development branch (always build/test/parity green)
├─ releases/0.11.0          <- 0.11.x release branch (tagged, published)
│   ├─ v0.11.0              <- first release tag (March 2026, packages not published then)
│   ├─ v0.11.0+rs.1         <- internal patch tag
│   └─ v0.11.0+rs.6         <- first registry publish
├─ custom/0.11.0            <- extension line (-ext.N tags, never published)
├─ releases/0.12.0          <- current 0.12.x line (v0.12.0, v0.12.1)
├─ custom/0.12.0            <- current extension line (tag v0.12.0-ext.2 preserved)
├─ port/0.12.0              <- ELK 0.12.0 porting work branch
├─ feature/*                <- elk-rs feature development (NAPI, optimizations, etc.)
└─ fix/*                    <- bug fixes
```

**Rules:**
- `main` always passes build/test/parity — development happens here
- `releases/X.Y.Z` branches are created from `main` when ready to release
  - New stable tags (`v<package-version>`) are placed on release branches; existing `+rs.N` tags remain historical
  - Hotfixes go to the release branch, then cherry-pick to `main` if needed
  - Release branches are long-lived (not deleted after release)
- `custom/X.Y.Z` branches integrate custom features on top of `main`
  - For a new ELK baseline, preserve custom history by merging the new release into a new integration branch created from the previous custom line.
  - New extension tags (`v<stable-package-version>-ext.N`) are placed on custom integration branches; the branch name may still identify the Java baseline
  - Individual features are developed on `custom/{feature-name}` branches, merged into `custom/X.Y.Z`
  - See `CUSTOM_FEATURES.md` for feature documentation
- New ELK version porting happens on `port/X.Y.Z` branches, merged to `main`
- Feature/fix branches fork from `main`, merge via PR

### Tag Strategy

The existing tag history is preserved. The two future examples below use the
new rules; they are not additional tags created by the policy change.

```
releases/0.11.0:
  v0.11.0                 <- stable release tag (ELK 0.11.0 parity; registries were not used yet)
  v0.11.0+rs.1            <- elk-rs internal hotfix (build only)
  v0.11.0+rs.2            <- elk-rs internal hotfix (build only)
  v0.11.0+rs.3            <- memory lifetime fixes (build only)
  v0.11.0+rs.4            <- MinWidthLayerer tie-order fix (build only)
  v0.11.0+rs.5            <- NAPI CI fixes (build only)
  v0.11.0+rs.6            <- first crates.io + npm publish of 0.11.0 (2026-10-19)

custom/0.11.0:
  v0.11.0-ext.1           <- extension release (custom features; build only)
  v0.11.0-ext.2           <- extension release (+ elk-live; build only)
  v0.11.0-ext.5           <- incorporates +rs.3 memory fixes (build only)
  v0.11.0-ext.6           <- incorporates +rs.4 MinWidthLayerer fix (tag only; never published)

releases/0.12.0:
  v0.12.0                 <- initial ELK 0.12.0 port tag
  v0.12.1                 <- published Rust fixes, still based on Java 0.12.0
  v0.12.2                 <- next published patch (future example)

custom/0.12.0:
  v0.12.0-ext.1           <- first extension tag at 371e186d (build only)
  v0.12.0-ext.2           <- existing tag at 3029a99b; legacy name, preserved
  v0.12.1-ext.1           <- next custom version if stable base remains 0.12.1 (future example)
```

**Rules:**
- New stable release: `v<package-version>`; new custom release:
  `v<stable-package-version>-ext.N`. The tag must match the manifests.
- Use annotated tags and preserve all existing tag targets. Do not create new
  `+rs.N` tags; use commit/CI identity for intermediate builds.
- Historical naming correction: `v0.12.0+rs.7` was corrected to `v0.12.0` before
  this transition. It does not authorize renaming any currently preserved tag.
- A pushed stable tag enables publication of missing package versions. Extension
  tags only build artifacts. Legacy workflow behavior is described in § 1.
- Before the next release, carry this policy revision and its matching checklist
  to the target release branch; retain branch-specific validation and state records.

---

## 4. Porting Workflow (New ELK Version)

### Step-by-Step Procedure

```
1. Prepare        Submodule update + diff analysis
2. Port           Apply Java changes to Rust
3. Verify         Pass all parity gates
4. Release        Version update + tag + publish
```

### Step 1: Prepare

```sh
# Create porting branch
git checkout -b port/0.12.0 main

# Update submodules to new release tags
git -C external/elk checkout v0.12.0
git -C external/elkjs checkout 0.12.0  # if elkjs has released a matching version

# Review Java changes
git -C external/elk diff v0.11.0..v0.12.0 --stat
git -C external/elk diff v0.11.0..v0.12.0 -- plugins/  # per-plugin changes

# Analyze change impact -> record in HISTORY.md
```

### Step 2: Port

Porting order (by dependency):
1. `org.eclipse.elk.graph` (data model changes)
2. `org.eclipse.elk.core` (core options/engine changes)
3. `org.eclipse.elk.alg.common` (common utilities)
4. Individual algorithms (`layered`, `force`, `mrtree`, `radial`, `disco`, `rectpacking`, `spore`)
5. `org.eclipse.elk.graph.json` (JSON importer/exporter)
6. `org.eclipse.elk.wasm` / `org.eclipse.elk.napi` (bindings, usually unchanged)
7. `org.eclipse.elk.js` (JS API, usually unchanged)

After porting each crate:
```sh
cargo build --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
```

### Step 3: Verify (Full Gates)

Run the full validation flow from `TESTING.md` § 3.6:

```sh
# 1. Phase wiring parity
LAYERED_PHASE_WIRING_PARITY_STRICT=true sh scripts/check_layered_phase_wiring_parity.sh

# 2-4. Build + Clippy + Test
cargo build --workspace && cargo clippy --workspace --all-targets && cargo test --workspace

# 5. Full model parity (new Java baseline required)
sh scripts/run_model_parity_elk_vs_rust.sh external/elk-models tests/model_parity_full

# 6. JS tests + both explicit backend parity runs from TESTING.md § 3.4
# Use fresh local backends and a Java 0.12.0 export; --require-java is required.

# 7. Phase-step trace verification (required alongside model parity)
python3 scripts/compare_phase_traces.py <java_trace_dir> <rust_trace_dir> --batch
```

### Step 4: Release

Historical initial-port example: `v0.12.0` already exists. Do not rerun these
commands for that version; use the actual next version and preserve existing refs.

```sh
# Merge porting branch to main
git checkout main
git merge port/0.12.0

# Create release branch from main
git checkout -b releases/0.12.0

# Set the package version everywhere (Cargo.toml, Cargo.lock, package.json files, lockfiles)
python3 scripts/bump_version.py 0.12.0
git commit -am "release: prepare elk-rs 0.12.0"

# First release of this base version: no +rs suffix
git tag -a v0.12.0 -m "elk-rs 0.12.0 (ELK 0.12.0 parity)"

# Push release branch and the tag: release.yml publishes npm + crates.io and creates the GitHub Release
git push origin releases/0.12.0
git push origin v0.12.0

# Return to main for continued development
git checkout main
```

### Hotfix on Release Branch

Example for the next stable patch; run the required gates before tagging:

```sh
git checkout releases/0.12.0
# Apply the fix and record its validation.
python3 scripts/bump_version.py 0.12.2
git commit -am "release: prepare elk-rs 0.12.2"
git tag -a v0.12.2 -m "elk-rs 0.12.2 (Java ELK 0.12.0 reference)"
git push github releases/0.12.0
git push github refs/tags/v0.12.2
# Synchronize the authorized mirrors with these exact refs.
# Integrate the fix into main if it is not already there.
```

For an intermediate build, use the commit SHA and CI run/attempt instead of
creating a `+rs.N` tag. No command in this example is executed by the policy update.

---

## 5. Customization Policy

elk-rs follows a 1:1 porting principle from Java ELK, but allows differences in three categories.

### Allowed Differences (documentation required)

| Category | Policy | Example |
|----------|--------|---------|
| **Java ELK bug avoidance** | Do not replicate Java bugs. Record rationale in HISTORY.md | `213_componentsCompaction` NaN propagation |
| **Idiomatic Rust conversion** | Express identical behavior using Rust idioms | Iterator, enum dispatch, ownership |
| **elk-rs exclusive features** | Additional features not in upstream (maintain API compatibility) | NAPI addon, WASM optimization, additional CLI |

### Prohibited Differences

| Category | Reason |
|----------|--------|
| Algorithm behavior changes | Breaks parity, unpredictable for users |
| API-incompatible changes | Cannot serve as elkjs drop-in replacement |
| Option/property semantic changes | Existing graph configurations produce different results |

### Customization Recording Rules

1. elk-rs exclusive changes must be recorded in `HISTORY.md` with `[elk-rs only]` tag
2. Java ELK bug avoidances are recorded with `[java-bug]` tag, referencing the Java issue number
3. If upstream later adopts the same change, update the tag to `[upstream-merged]`
4. Maintain a list of allowed differences in the "Known Drifts" section of `tests/PARITY.md`

---

## 6. Registry Publishing

### What is published

| Registry | Packages | Version | Trigger |
|----------|----------|---------|---------|
| npm | `@openedges/elk-layout` (WASM + loader) and `@openedges/elk-layout-darwin-arm64`, `@openedges/elk-layout-darwin-x64`, `@openedges/elk-layout-linux-x64-gnu`, `@openedges/elk-layout-linux-x64-musl`, `@openedges/elk-layout-linux-arm64-gnu`, `@openedges/elk-layout-win32-x64-msvc` | package version, dist-tag `latest` | `publish-npm` job of `.github/workflows/release.yml` on pushed stable tags with a prerelease-free package version |
| crates.io | `elk-layout` (facade, `crates/elk-layout`) and the 17 library crates `org-eclipse-elk-*` (everything except the `napi`/`wasm` bindings) | package version | `publish-crates` job of `.github/workflows/release.yml` on pushed stable tags with a prerelease-free package version |

Never published: `-ext.N` tags, `org-eclipse-elk-napi`/`org-eclipse-elk-wasm`
(`publish = false`), `elk-layout-napi` and `elk-rs-live` (`"private": true`).

A successful stable tag run attaches the six `elk-layout.<triple>.node` binaries and
`elk-layout-<version>-wasm.tar.gz` to a GitHub Release with generated release notes
(`release` job). `-ext.N` tags build the binaries too but keep them as workflow artifacts:
extension builds never appear on the public release page. A manual run of `release.yml`
builds everything and dry-runs both publish scripts; its `publish_npm`/`publish_crates`
inputs request a real publish of the missing versions (start the run on the release tag
when the environments are limited to tags).

Names: `elk-rs` is taken on crates.io by an unrelated crate, so the Rust facade is
`elk-layout`; the npm packages live under the `@openedges` organization. The GitHub
repository keeps its `elk-rs` name.

### Scripts

- `sh scripts/publish_npm.sh [--dry-run] [--expect-version V]`: platform packages first,
  then `@openedges/elk-layout`; skips `package@version` pairs already on npm; refuses to publish a
  platform package without its `.node` binary or the main package without both browser `dist/wasm/` and Node.js `dist/wasm-node/` bundles.
- `python3 scripts/publish_crates.py [--dry-run] [--no-verify] [--expect-version V]`:
  one `cargo publish -p ...` call for the crates missing on crates.io (cargo orders them
  and waits for the index), retrying after crates.io's new-crate rate limit.
- Both take the package version from CI (`v0.12.2` -> `0.12.2`) and fail if any
  manifest disagrees. Legacy `+rs.N` stripping remains supported as described in § 1;
  new releases use bare stable tags.

### Credentials: Trusted Publishing only

GitHub Actions publishes through OIDC without stored npm or crates.io tokens.
Do not register `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN` as GitHub Secrets. The registry
trusts the identity "repository `openedges/elk-rs`, workflow file `release.yml`,
GitHub environment". The existing seven npm packages and 18 crates used this
identity for the 0.12.0 publication (run39357332958).

| Job | Authentication | Failure behavior |
|-----|----------------|------------------|
| `publish-npm` | `id-token: write`; npm 11 automatically exchanges the GitHub identity | Publish fails; no `.npmrc` token injection or stored-token retry |
| `publish-crates` | `id-token: write`; `rust-lang/crates-io-auth-action@v1` obtains a short-lived token and revokes it after the job | Authentication or publish failure fails the job |

The `CARGO_REGISTRY_TOKEN` environment variable in `publish-crates` is the action's
short-lived output, **not a stored secret**. No credential is needed for public
registry lookups or publish dry runs. An owner registers each package/crate's
trusted publisher once; new package versions reuse that registration:

| Registry | Where | Fields |
|----------|-------|--------|
| npmjs.com, 7 packages | package page → Settings → Trusted publishing → GitHub Actions | Organization or user `openedges`, Repository `elk-rs`, Workflow filename `release.yml`, Environment name `npm`; Allowed actions must include `npm publish` |
| crates.io, 18 crates | crate page → Settings → Trusted Publishing → Add → GitHub | Repository owner `openedges`, Repository name `elk-rs`, Workflow filename `release.yml`, Environment `crates-io` |

- GitHub environments `npm` and `crates-io` allow only `v*` tags, as verified on
  2026-10-20. A manual real publish must therefore be dispatched on a release tag.
  A branch run with both publish inputs false remains a dry run. Prerelease/extension
  versions reject either real-publish input in preflight, including tag dispatches.
- npm expires a **new** trusted-publisher registration if its first successful publish
  does not occur within two days. The existing registrations have completed that
  first publish; do not recreate them for each version.
- npm trusted publishing needs npm 11.5.1+ on Node 22.14+; `release.yml` installs `npm@11`
  on the Node version pinned in `mise.toml`. Provenance requires `repository.url` of every
  npm package to be `git+https://github.com/openedges/elk-rs.git`, which the manifests carry.
- For an OIDC error, check the package's owner/repository/workflow/environment fields
  and `id-token: write`, repair the registration, then rerun the failed job.
  Do not restore a long-lived-token fallback. Already published versions are skipped.
- New package/crate names need a separate bootstrap and trusted-publisher registration.
  Existing packages need neither `npm login` nor `cargo login` for CI releases.
- Removing a GitHub Secret does not revoke the corresponding token at npm/crates.io.
  Registry account token revocation and npm's "Require two-factor authentication and
  disallow tokens" setting are separate account operations; this workflow does not
  depend on either setting.

References: [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/),
[crates.io auth action](https://github.com/rust-lang/crates-io-auth-action).
The initial token-based bootstrap and migration evidence are recorded in `HISTORY.md`.
