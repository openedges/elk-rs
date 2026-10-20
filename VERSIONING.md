# elk-rs Versioning and Operations Policy

## Context

elk-rs is a Rust port of Java ELK (Eclipse Layout Kernel) and also provides an elkjs-compatible npm package. Current state:

- **ELK Java** (`external/elk`): pinned to `v0.12.0`, commit `323312916048544e4e594eab3303cdb0d01e2929`
- **elkjs** (`external/elkjs`): pinned to `0.12.0`, commit `ff5771d7165445c42c408bb8a090c8035272218c`
- **elk-models** (`external/elk-models`): pinned to the parity corpus commit
- **Working tree**: all Rust crates and npm packages `0.12.0`; the JS test dependency is exact `elkjs@0.12.0`
- **Published package version**: `0.12.0`; release workflow 39357332958 completed crates.io, npm and GitHub Release publication on 2026-10-20
- **Tag correction pending**: the first 0.12.0 release should be `v0.12.0`. It was mistakenly published under `v0.12.0+rs.7`; that existing tag/Release has not been changed. The first subsequent Rust revision is `v0.12.0+rs.1`.
- **Verification**: warning-free build/Clippy, 1,032 Rust tests passed (0 failed, 2 ignored); fresh NAPI and browser WASM each match Java on 550/550 JSON models. Full model and phase-step trace status: [upgrade report](tests/upgrade_0_12_0.md)
- **Node.js runtime**: the 0.12.0 npm package requires Node.js 18+ (minimum 18.0.0 verified); development and release builds use pinned Node.js 24. Node.js 16 is unsupported because the standard WASM artifact requires reference types unavailable there without experimental flags.
- **License**: `EPL-2.0 OR GPL-3.0-or-later`, with the upstream conditional secondary-license notice preserved in `LICENSE` and `NOTICE`


This policy keeps versions aligned across all three projects while managing porting, verification, and releases systematically.

---

## 1. Version Management

### Principle: ELK Version Alignment With an elk-rs Patch Offset

elk-rs `MAJOR.MINOR` always matches the target ELK Java version. `PATCH` starts at
the ELK patch number and then **counts elk-rs's own published fixes**, the same way
elkjs publishes `0.9.2`, `0.9.3`, `0.10.1`, `0.10.2` and `0.11.1` on top of the ELK
releases `0.9.1`, `0.10.0` and `0.11.0`.

```
elk-rs 0.11.0  =  ELK Java 0.11.0 port          (first publish of the 0.11 line)
elk-rs 0.11.1  =  ELK Java 0.11.0 + elk-rs fixes (example patch version)
elk-rs 0.12.0  =  ELK Java 0.12.0 port
```

| Project | Working-tree target | Latest elk-rs package release |
|---------|---------------------|-------------------------------|
| ELK Java reference | 0.12.0 | 0.12.0 |
| elkjs reference | 0.12.0 | 0.12.0 |
| elk-rs (Rust crates) | 0.12.0 | 0.12.0 |
| elk-rs (npm) | 0.12.0 | 0.12.0 |

**Rules:**
- A new ELK `MAJOR.MINOR` port resets `PATCH` to ELK's patch number (`0.12.0`)
- Publishing elk-rs fixes for the current line bumps `PATCH` (`0.11.0` -> `0.11.1` -> ...)
- If ELK itself releases a patch (`0.11.1`) after elk-rs has used that number, the port
  takes the next free elk-rs patch number and the CHANGELOG names the ELK version it ports
- All Rust workspace crates and the npm packages share the same version, always

### elk-rs Internal Change Management

The first release of each base version is tagged **`v<version>` without a suffix**.
Only subsequent Rust revisions of that same base version use `v<version>+rs.<N>`,
starting at `N = 1`. The revision counter belongs to one base version; it never
carries over to another version. A new base version starts with a bare tag again.

```
v0.12.0          <- first release of base version 0.12.0
v0.12.0+rs.1     <- first subsequent Rust revision of 0.12.0
v0.12.0+rs.2     <- second subsequent Rust revision of 0.12.0
v0.12.1          <- first release of new package version 0.12.1
v0.12.1+rs.1     <- first subsequent Rust revision of 0.12.1
v0.13.0          <- first release of the next ELK port
```

Tag naming and registry publication are separate: `release.yml` handles both bare
release tags and revision tags, and publishes only package versions not already
present in the registry.

- **Build metadata never reaches a registry.** `npm publish` strips `+rs.N`
  (`semver.clean("0.11.0+rs.5")` is `0.11.0`), and crates.io rejects a second version
  that differs only in build metadata (uniqueness is on the version without `+build`).
  Only the package version in the manifests matters; `+rs.N` lives in git tags and the
  CHANGELOG.
- **Publishing a fix = bump the patch version, then tag.** `python3 scripts/bump_version.py 0.12.1`
  updates every manifest and lockfile (see Version Sync Files), the quality gates run,
  the commit is tagged `v0.12.1` (the first release of that new base version) and pushed. `release.yml` then publishes every
  package/crate whose version is not on the registry yet and attaches the binaries to a
  GitHub Release.
- **A tag without a bump is a build-only tag.** Both publish scripts skip
  package@version pairs that already exist, so tagging a later `v0.11.0+rs.N` after `0.11.0`
  has been published builds the NAPI binaries and publishes nothing.
- **Prereleases do not work for fixes.** `0.11.0-rs.1` sorts *below* `0.11.0`, so once
  `0.11.0` is published no `^0.11.0` consumer would receive it. Prerelease versions are
  reserved for publishing *before* the first stable version of a line, if ever needed.
- **Extension tags are never published.** `v0.11.0-ext.N` tags on `custom/*` branches
  build but skip the publish and release jobs: `release.yml` publishes only when the
  manifests' version, which the tag must match, has no prerelease part.
- **CHANGELOG.md**: elk-rs changes are grouped by published version

#### CHANGELOG Rules

The changelog follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
conventions adapted for ELK version alignment.

**Structure:**
- One `## [X.Y.Z]` section per published version (ELK port or elk-rs patch)
- Within each version, use standard section headers: `### Added`, `### Fixed`,
  `### Changed`, `### Removed`, `### ELK Porting`
- `### ELK Porting` is elk-rs specific: summarizes porting scope and parity metrics
- Build-only `+rs.N` tags between published versions are grouped under
  `### elk-rs Internal Changes (v0.11.0+rs.1 ~ v0.11.0+rs.N)` of the version they
  ship with

**Writing guidelines:**
- Each entry starts with a verb in past tense (Added, Fixed, Removed, etc.)
- Keep entries concise (1-2 lines). For complex changes, reference `HISTORY.md`
- Include parity metrics when relevant (e.g., "1438/1438 model parity")
- Record known issues and accepted exceptions in `### Known Issues`
- Do NOT duplicate full `HISTORY.md` content — CHANGELOG is user-facing summary,
  HISTORY.md is developer-facing detailed log

**When to update:**
- On each new published package version (first tag `v<version>`)
- On each build-only `+rs.N` tag (add entry under Internal Changes)
- Accumulated internal changes are folded into the next published version's section

**Example:**

```markdown
## [0.11.1] - YYYY-MM-DD

### Fixed
- ...

### elk-rs Internal Changes (v0.11.1+rs.1 ~ v0.11.1+rs.2)
- [bugfix] Improved WASM fallback error message
- [perf] 20% layered algorithm performance improvement

## [0.12.0] - YYYY-MM-DD

### ELK Porting
- Full ELK 0.12.0 porting complete (N/N parity)
```

### Version Sync Files

`python3 scripts/bump_version.py <version>` edits the manifest and lockfile rows below;
`--check` verifies their package versions agree. Update `CHANGELOG.md` manually.
The script does not change the `elkjs` development dependency: pin that explicitly
to the target upstream version and regenerate the npm lockfile.

| File | Version Field |
|------|---------------|
| `Cargo.toml` (workspace root) | `[workspace.package] version` and the `version = "=X.Y.Z"` pins in `[workspace.dependencies]` |
| `Cargo.lock` | workspace member entries (`cargo update --workspace`) |
| `plugins/org.eclipse.elk.js/package.json` | `"version"` and the six `optionalDependencies` on `@openedges/elk-layout-*` |
| `plugins/org.eclipse.elk.js/package-lock.json` | root package `"version"` entries |
| `plugins/org.eclipse.elk.js/npm/*/package.json` | `"version"` of the six platform packages |
| `plugins/org.eclipse.elk.napi/package.json` | `"version"` (private helper manifest for `napi build`) |
| `packages/elk-live/package.json` + `package-lock.json` | `"version"` (private) |
| `CHANGELOG.md` | Change history per published version |

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
├─ releases/0.12.0          <- current release line (initial tag correction pending)
├─ port/0.12.0              <- ELK 0.12.0 porting work branch
├─ feature/*                <- elk-rs feature development (NAPI, optimizations, etc.)
└─ fix/*                    <- bug fixes
```

**Rules:**
- `main` always passes build/test/parity — development happens here
- `releases/X.Y.Z` branches are created from `main` when ready to release
  - Initial tags (`v<version>`) and subsequent revision tags (`v<version>+rs.<N>`) are placed on release branches
  - Hotfixes go to the release branch, then cherry-pick to `main` if needed
  - Release branches are long-lived (not deleted after release)
- New ELK version porting happens on `port/X.Y.Z` branches, merged to `main`
- Feature/fix branches fork from `main`, merge via PR

### Tag Strategy

```
releases/0.11.0:
  v0.11.0                 <- stable release tag (ELK 0.11.0 parity; registries were not used yet)
  v0.11.0+rs.1            <- elk-rs internal hotfix (build only)
  v0.11.0+rs.2            <- elk-rs internal hotfix (build only)
  v0.11.0+rs.3            <- memory lifetime fixes (build only)
  v0.11.0+rs.4            <- MinWidthLayerer tie-order fix (build only)
  v0.11.0+rs.5            <- NAPI CI fixes (build only)
  v0.11.0+rs.6            <- first crates.io + npm publish of 0.11.0 (2026-10-19)

releases/0.12.0:
  v0.12.0                 <- required initial ELK 0.12.0 port tag (correction pending)
  v0.12.0+rs.1            <- first subsequent Rust revision (future)
  v0.12.0+rs.2            <- second subsequent Rust revision (future)
```

**Rules:**
- First release of a base version: `v<package version>` on the release branch, with no `+rs` suffix
- Subsequent revisions of that base version: `+rs.1`, `+rs.2`, ...; a new base version starts with a bare tag again
- Historical naming error: `v0.12.0+rs.7` was published before this correction. It is not the numbering rule for the 0.12.0 line; its tag/Release correction is pending
- Whether a tag publishes depends only on whether `<package version>` is already on the
  registries (the publish scripts skip existing versions)
- Use annotated tags: `git tag -a v0.11.1 -m "elk-rs 0.11.1 (ELK 0.11.0 + fixes)"`
- Extension tags `v<version>-ext.<N>` live on `custom/*` branches and are never published

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

```sh
# Work on the release branch
git checkout releases/0.11.0

# Apply fix, commit
git commit -m "fix: <description>"

# Build-only: tag without bumping (nothing is published because 0.11.0 is already out)
git tag -a v0.11.0+rs.7 -m "elk-rs 0.11.0+rs.7 internal patch"

# Published fix: bump the patch version first, then tag
python3 scripts/bump_version.py 0.11.1
git commit -am "release: prepare elk-rs 0.11.1"
git tag -a v0.11.1 -m "elk-rs 0.11.1 (ELK 0.11.0 + fixes)"

# Push
git push origin releases/0.11.0 --tags

# Cherry-pick to main if applicable
git checkout main
git cherry-pick <commit-hash>
```

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
| npm | `@openedges/elk-layout` (WASM + loader) and `@openedges/elk-layout-darwin-arm64`, `@openedges/elk-layout-darwin-x64`, `@openedges/elk-layout-linux-x64-gnu`, `@openedges/elk-layout-linux-x64-musl`, `@openedges/elk-layout-linux-arm64-gnu`, `@openedges/elk-layout-win32-x64-msvc` | package version, dist-tag `latest` | `publish-npm` job of `.github/workflows/release.yml` on `v*` tags without `-ext.` |
| crates.io | `elk-layout` (facade, `crates/elk-layout`) and the 17 library crates `org-eclipse-elk-*` (everything except the `napi`/`wasm` bindings) | package version | `publish-crates` job of `.github/workflows/release.yml` on `v*` tags without `-ext.` |

Never published: `-ext.N` tags, `org-eclipse-elk-napi`/`org-eclipse-elk-wasm`
(`publish = false`), `elk-layout-napi` and `elk-rs-live` (`"private": true`).

The same tag run attaches the six `elk-layout.<triple>.node` binaries and
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
- Both take the tag's base version from CI (`v0.12.0` or `v0.12.0+rs.1` -> `0.12.0`) and fail if any
  manifest disagrees.

### Credentials: Trusted Publishing

The first publish (`0.11.0`, 2026-10-19, tag `v0.11.0+rs.6`) used long-lived tokens: the
repository secrets `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN`, plus a local `cargo login` for
the first five crates (crates.io limits *new* crates to a burst of five, then one every
ten minutes; the tag run published the other 13 through the retry loop of
`publish_crates.py` in 2 h 7 min). Every later publish runs through GitHub Actions OIDC,
which both registries call Trusted Publishing: the registry trusts the identity
"repository `openedges/elk-rs`, workflow file `release.yml`, GitHub environment", and the
job receives a short-lived token (crates.io: 30 minutes). A registration is made per
package and per crate, by an owner, and only for a package/crate that already exists:

| Registry | Where | Fields |
|----------|-------|--------|
| npmjs.com, 7 packages | package page → Settings → Trusted publishing → GitHub Actions | Organization or user `openedges`, Repository `elk-rs`, Workflow filename `release.yml`, Environment name `npm`; Allowed actions must include `npm publish` |
| crates.io, 18 crates | crate page → Settings → Trusted Publishing → Add → GitHub | Repository owner `openedges`, Repository name `elk-rs`, Workflow filename `release.yml`, Environment `crates-io` |

- GitHub environments `npm` and `crates-io` (repository Settings → Environments). GitHub
  creates them on first use, but creating them by hand allows a deployment rule that limits
  each to `v*` tags, so no branch run can reach a registry (a manual publish is then
  dispatched on the release tag).
- npm expires a trusted-publisher registration that has not completed a publish within two
  days, so the seven npm registrations are made at most two days before the first
  publishing tag (TESTING.md § 3.6). crates.io registrations do not expire.
- Transition: `release.yml` still honours the two secrets when the OIDC publish fails
  (a package or crate without a registration; crates.io scopes its token to the registered
  crates, so the unregistered ones get 403 and the job retries them with the secret and
  warns). After the first release that published
  through OIDC, delete `NPM_TOKEN` and `CARGO_REGISTRY_TOKEN` and set every npm package's
  Publishing access to "Require two-factor authentication and disallow tokens".
- npm trusted publishing needs npm 11.5.1+ on Node 22.14+; `release.yml` installs `npm@11`
  on the Node version pinned in `mise.toml`. Provenance requires `repository.url` of every
  npm package to be `git+https://github.com/openedges/elk-rs.git`, which the manifests carry.
- All 18 crates.io registrations must exist before `CARGO_REGISTRY_TOKEN` is deleted; until
  then a partial set only costs the warning above.
- New crate names (a future plugin crate) hit the new-crate limit (a burst of five, then
  one every ten minutes), which the 30-minute OIDC token cannot wait out: publish them
  locally after `cargo login` first and let the tag run skip them.
