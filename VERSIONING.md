# elk-rs Versioning and Operations Policy

## Context

elk-rs is a Rust port of Java ELK (Eclipse Layout Kernel) and also provides an elkjs-compatible npm package. Current state:

- **ELK Java** (`external/elk`): pinned to `v0.11.0` release tag
- **elkjs** (`external/elkjs`): pinned to `0.11.0` release tag
- **elk-models** (`external/elk-models`): pinned to parity-verified commit
- **elk-rs**: all Rust crates `0.11.0`, npm `elk-rs@0.11.0`
- **Latest internal patch**: Git tag `v0.11.0+rs.5` (NAPI CI fixes: aarch64 cross linker, Intel macOS cross build; package versions unchanged)
- **Registries**: crates.io `org-eclipse-elk-*` (17 library crates) and npm `elk-rs` + `@elk-rs/*` (6 platform packages). First publish pending: `0.11.0` goes out with the next `+rs.N` tag once the npm/crates.io accounts and CI tokens exist (see § 6)
- **Parity**: 1438/1438 models 100% match (ELK 0.11.0 porting complete)

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
elk-rs 0.11.1  =  ELK Java 0.11.0 + elk-rs fixes (next published +rs.N tag)
elk-rs 0.12.0  =  ELK Java 0.12.0 port
```

| Project | Current | Next |
|---------|---------|------|
| ELK Java | 0.11.0 | 0.12.0 |
| elkjs | 0.11.0 | 0.12.0 |
| elk-rs (Rust crates) | 0.11.0 | 0.11.1 (fixes) or 0.12.0 (port) |
| elk-rs (npm) | 0.11.0 | 0.11.1 (fixes) or 0.12.0 (port) |

**Rules:**
- A new ELK `MAJOR.MINOR` port resets `PATCH` to ELK's patch number (`0.12.0`)
- Publishing elk-rs fixes for the current line bumps `PATCH` (`0.11.0` -> `0.11.1` -> ...)
- If ELK itself releases a patch (`0.11.1`) after elk-rs has used that number, the port
  takes the next free elk-rs patch number and the CHANGELOG names the ELK version it ports
- All Rust workspace crates and the npm packages share the same version, always

### elk-rs Internal Change Management

Every elk-rs change on a release branch is tagged `v<package version>+rs.<N>`; `N` is a
counter that keeps increasing across patch bumps (`v0.11.0+rs.5`, `v0.11.0+rs.6`,
`v0.11.1+rs.7`, ...). The tag decides whether anything is published:

```
git: v0.11.0+rs.5        <- package version 0.11.0 already on the registries? no  -> publish 0.11.0
git: v0.11.0+rs.6        <- package version still 0.11.0 and already published   -> build only
git: v0.11.1+rs.7        <- package version bumped to 0.11.1                      -> publish 0.11.1
```

- **Build metadata never reaches a registry.** `npm publish` strips `+rs.N`
  (`semver.clean("0.11.0+rs.5")` is `0.11.0`), and crates.io rejects a second version
  that differs only in build metadata (uniqueness is on the version without `+build`).
  Only the package version in the manifests matters; `+rs.N` lives in git tags and the
  CHANGELOG.
- **Publishing = bump the patch version, then tag.** `python3 scripts/bump_version.py 0.11.1`
  updates every manifest and lockfile (see Version Sync Files), the quality gates run,
  the commit is tagged `v0.11.1+rs.<N>` and pushed. `napi.yml` and `cargo-publish.yml`
  then publish every package/crate whose version is not on the registry yet.
- **A tag without a bump is a build-only tag.** Both publish scripts skip
  package@version pairs that already exist, so tagging `v0.11.0+rs.6` after `0.11.0`
  has been published builds the NAPI binaries and publishes nothing.
- **Prereleases do not work for fixes.** `0.11.0-rs.1` sorts *below* `0.11.0`, so once
  `0.11.0` is published no `^0.11.0` consumer would receive it. Prerelease versions are
  reserved for publishing *before* the first stable version of a line, if ever needed.
- **Extension tags are never published.** `v0.11.0-ext.N` tags on `custom/*` branches
  build but skip the publish jobs (`!contains(github.ref_name, '-ext.')`).
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
- On each published version (`v<version>+rs.<N>` that bumps the package version)
- On each build-only `+rs.N` tag (add entry under Internal Changes)
- Accumulated internal changes are folded into the next published version's section

**Example:**

```markdown
## [0.11.1] - YYYY-MM-DD

### Fixed
- ...

### elk-rs Internal Changes (v0.11.0+rs.6 ~ v0.11.0+rs.8)
- [bugfix] Improved WASM fallback error message
- [perf] 20% layered algorithm performance improvement

## [0.12.0] - YYYY-MM-DD

### ELK Porting
- Full ELK 0.12.0 porting complete (N/N parity)
```

### Version Sync Files

`python3 scripts/bump_version.py <version>` edits all of them; `--check` verifies they agree.

| File | Version Field |
|------|---------------|
| `Cargo.toml` (workspace root) | `[workspace.package] version` and the `version = "=X.Y.Z"` pins in `[workspace.dependencies]` |
| `Cargo.lock` | workspace member entries (`cargo update --workspace`) |
| `plugins/org.eclipse.elk.js/package.json` | `"version"` and the six `optionalDependencies` on `@elk-rs/*` |
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
version = "0.11.0"
license = "EPL-2.0"
edition = "2021"
repository = "https://github.com/openedges/elk-rs"

[workspace.dependencies]
org-eclipse-elk-graph = { path = "plugins/org.eclipse.elk.graph", version = "=0.11.0" }

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

- The pins are exact (`=0.11.0`) because the crates are released together and share
  internal APIs; a consumer can never mix two elk-rs versions.
- `[dev-dependencies]` stay path-only on purpose. Cargo drops path-only
  dev-dependencies from the published manifest, which is what lets the test-only cycles
  (`core <-> alg-*`, `alg-common <-> alg-*`) publish.
- `org-eclipse-elk-napi` and `org-eclipse-elk-wasm` are `publish = false`: they are
  cdylib bindings shipped through npm, not crates.io.

---

## 2. Submodule Pinning Policy

### Submodule Commit Pinning Rules

| Submodule | Pin Target | Rationale |
|-----------|------------|-----------|
| `external/elk` | Release tag (e.g., `v0.11.0`) | Parity reference point, reproducibility |
| `external/elkjs` | Release tag (e.g., `0.11.0`) | JS parity reference point |
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
│   └─ v0.11.0+rs.5         <- internal patch tag
├─ custom/0.11.0            <- extension line (-ext.N tags, never published)
├─ releases/0.12.0          <- next release branch (future)
├─ port/0.12.0              <- ELK 0.12.0 porting work branch
├─ feature/*                <- elk-rs feature development (NAPI, optimizations, etc.)
└─ fix/*                    <- bug fixes
```

**Rules:**
- `main` always passes build/test/parity — development happens here
- `releases/X.Y.Z` branches are created from `main` when ready to release
  - Release tags (`v<version>+rs.<N>`) are placed on release branches
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
  v0.11.0+rs.6            <- first crates.io + npm publish of 0.11.0 (planned)
  v0.11.1+rs.7            <- next published fix: patch bump + publish (future)

releases/0.12.0:
  v0.12.0+rs.N            <- ELK 0.12.0 port, published as 0.12.0
```

**Rules:**
- Release tags: `v<package version>+rs.<N>` on the release branch; `N` never resets
- Whether a tag publishes depends only on whether `<package version>` is already on the
  registries (the publish scripts skip existing versions)
- Use annotated tags: `git tag -a v0.11.1+rs.7 -m "elk-rs 0.11.1 (ELK 0.11.0 + fixes)"`
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

# 6. JS parity
cd plugins/org.eclipse.elk.js && npm test && npm run test:parity

# 7. Phase-step trace verification (optional, if drift occurs)
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

# Tag on the release branch (first +rs.N of the line continues the counter)
git tag -a v0.12.0+rs.9 -m "elk-rs 0.12.0 (ELK 0.12.0 parity)"

# Push release branch and the tag: napi.yml publishes npm, cargo-publish.yml publishes crates.io
git push origin releases/0.12.0
git push origin v0.12.0+rs.9

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
git tag -a v0.11.0+rs.6 -m "elk-rs 0.11.0+rs.6 internal patch"

# Published fix: bump the patch version first, then tag
python3 scripts/bump_version.py 0.11.1
git commit -am "release: prepare elk-rs 0.11.1"
git tag -a v0.11.1+rs.7 -m "elk-rs 0.11.1 (ELK 0.11.0 + fixes)"

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
| npm | `elk-rs` (WASM + loader) and `@elk-rs/darwin-arm64`, `@elk-rs/darwin-x64`, `@elk-rs/linux-x64-gnu`, `@elk-rs/linux-x64-musl`, `@elk-rs/linux-arm64-gnu`, `@elk-rs/win32-x64-msvc` | package version, dist-tag `latest` | `.github/workflows/napi.yml` on `v*` tags without `-ext.` |
| crates.io | the 17 library crates `org-eclipse-elk-*` (everything except the `napi`/`wasm` bindings) | package version | `.github/workflows/cargo-publish.yml` on `v*` tags without `-ext.` |

Never published: `-ext.N` tags, `org-eclipse-elk-napi`/`org-eclipse-elk-wasm`
(`publish = false`), `elk-rs-napi` and `elk-rs-live` (`"private": true`).

### Scripts

- `sh scripts/publish_npm.sh [--dry-run] [--expect-version V]`: platform packages first,
  then `elk-rs`; skips `package@version` pairs already on npm; refuses to publish a
  platform package without its `.node` binary or the main package without `dist/wasm/`.
- `python3 scripts/publish_crates.py [--dry-run] [--no-verify] [--expect-version V]`:
  one `cargo publish -p ...` call for the crates missing on crates.io (cargo orders them
  and waits for the index), retrying after crates.io's new-crate rate limit.
- Both take the tag's base version from CI (`v0.11.1+rs.7` -> `0.11.1`) and fail if any
  manifest disagrees.

### Credentials and first publish

- npm: an npm account that owns the `elk-rs` package name and the `@elk-rs` organization,
  and a granular access token with publish rights (repository secret `NPM_TOKEN`). The
  first publish of a package name happens from CI like every other one.
- crates.io: a crates.io API token with `publish-new` + `publish-update` (repository secret
  `CARGO_REGISTRY_TOKEN`; locally `cargo login`). crates.io limits *new* crates to a burst
  of five, then one every ten minutes, so the first publish of the 17 crates is run
  locally with `python3 scripts/publish_crates.py` (it sleeps and retries; or ask
  help@crates.io to raise the limit beforehand). Later versions are updates (burst of 30)
  and run from CI.
- Once the packages exist, both registries support Trusted Publishing (GitHub Actions
  OIDC), which removes the long-lived tokens; the workflows note where `id-token: write`
  would go.
