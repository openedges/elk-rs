# elk-rs Versioning and Operations Policy

## Context

elk-rs is a Rust port of Java ELK (Eclipse Layout Kernel) and also provides an elkjs-compatible npm package. Current state:

- **ELK Java** (`external/elk`): pinned to `v0.11.0` release tag
- **elkjs** (`external/elkjs`): pinned to `0.11.0` release tag
- **elk-models** (`external/elk-models`): pinned to parity-verified commit
- **elk-rs**: all Rust crates `0.11.0`, npm `elk-rs@0.11.0`
- **Parity**: 1438/1438 models 100% match (ELK 0.11.0 porting complete)

This policy keeps versions aligned across all three projects while managing porting, verification, and releases systematically.

---

## 1. Version Management

### Principle: Exact ELK Version Alignment

elk-rs version matches the target ELK Java version in `MAJOR.MINOR.PATCH`.

```
elk-rs 0.11.0  =  ELK Java 0.11.0  =  elkjs 0.11.0
```

| Project | Current | Next |
|---------|---------|------|
| ELK Java | 0.11.0 | 0.12.0 |
| elkjs | 0.11.0 | 0.12.0 |
| elk-rs (Rust crates) | 0.11.0 | 0.12.0 |
| elk-rs (npm) | 0.11.0 | 0.12.0 |

**Rules:**
- elk-rs `MAJOR.MINOR.PATCH` always matches the target ELK Java version
- When ELK releases `0.11.1`, elk-rs ports the changes and releases `0.11.1`
- All Rust workspace crates and the npm package share the same version

### elk-rs Internal Change Management

elk-rs-specific bugfixes and features are managed **without bumping the version**, using the following scheme.

#### Between-Release Changes: Build Metadata + CHANGELOG

```
npm: elk-rs@0.11.0       <- stable release (ELK 0.11.0 parity)
git: v0.11.0+rs.1        <- elk-rs internal patch #1
git: v0.11.0+rs.2        <- elk-rs internal patch #2
npm: elk-rs@0.11.0       <- npm version unchanged (no re-publish)
```

- **Git tags**: `v{ELK_VERSION}+rs.{N}` format to track elk-rs internal changes
  - Semver build metadata (`+`) has no effect on version precedence
  - Examples: `v0.11.0+rs.1`, `v0.11.0+rs.2`
- **No npm re-publish**: the npm package is not re-published for the same ELK version
  - Accumulated elk-rs fixes ship with the next ELK release (e.g., `0.11.1` or `0.12.0`)
  - Exception: critical fixes may be published as a prerelease: `0.11.0-rs.1`
- **CHANGELOG.md**: elk-rs changes are grouped by ELK version

#### Extension Releases: Pre-release Channel (`-ext.N`)

Custom features (not in upstream ELK Java) are released via a pre-release channel:

```
npm: elk-rs@0.11.0           <- stable release (ELK 0.11.0 parity)
npm: elk-rs@0.11.0-ext.1     <- extension release #1 (custom features)
npm: elk-rs@0.11.0-ext.2     <- extension release #2
```

- **Version format**: `{ELK_VERSION}-ext.{N}` — semver pre-release label `ext` (extension)
  - `ext` indicates the release is a superset of the stable version with additional features
  - Examples: `0.11.0-ext.1`, `0.11.0-ext.2`
- **Git tags**: `v{ELK_VERSION}-ext.{N}` on the `custom/{ELK_VERSION}` integration branch
- **Cargo + npm**: Both use the same `-ext.N` version — publishable to registries
  - `npm install elk-rs@0.11.0-ext.1` (explicit install; `npm install elk-rs` still gets stable)
  - `elk-rs = "=0.11.0-ext.1"` in Cargo.toml (exact match required)
- **Semver note**: Technically `0.11.0-ext.1 < 0.11.0` in semver ordering, but this is not a
  problem in practice since extension releases are always installed via explicit version specifier
- **Version bumping**: When `main` moves to the next ELK version, extension releases rebase and
  restart numbering (e.g., `0.12.0-ext.1`)
- **Branch**: Extension releases are tagged on `custom/{ELK_VERSION}` branches, not on `releases/*`
- **Documentation**: Custom features are documented in `CUSTOM_FEATURES.md`

#### CHANGELOG Rules

The changelog follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
conventions adapted for ELK version alignment.

**Structure:**
- One `## [X.Y.Z]` section per ELK version release
- Within each version, use standard section headers: `### Added`, `### Fixed`,
  `### Changed`, `### Removed`, `### ELK Porting`
- `### ELK Porting` is elk-rs specific: summarizes porting scope and parity metrics
- Between-release elk-rs internal changes are grouped under
  `### elk-rs Internal Changes (v0.11.0+rs.1 ~ v0.11.0+rs.N)`

**Writing guidelines:**
- Each entry starts with a verb in past tense (Added, Fixed, Removed, etc.)
- Keep entries concise (1-2 lines). For complex changes, reference `HISTORY.md`
- Include parity metrics when relevant (e.g., "1438/1438 model parity")
- Record known issues and accepted exceptions in `### Known Issues`
- Do NOT duplicate full `HISTORY.md` content — CHANGELOG is user-facing summary,
  HISTORY.md is developer-facing detailed log

**When to update:**
- On each stable release (`v{ELK_VERSION}`)
- On each `+rs.N` internal patch tag (add entry under Internal Changes)
- Accumulated internal changes are folded into the next release section

**Example:**

```markdown
## [0.12.0] - YYYY-MM-DD

### ELK Porting
- Full ELK 0.12.0 porting complete (N/N parity)
- New algorithm: ...

### Added
- ...

### Fixed
- ...

### elk-rs Internal Changes (v0.11.0+rs.1 ~ v0.11.0+rs.N)
- [bugfix] Improved WASM fallback error message
- [feature] Added NAPI darwin-arm64 addon
- [perf] 20% layered algorithm performance improvement
```

### Version Sync Files

| File | Version Field | Stable | Extension |
|------|---------------|--------|-----------|
| `Cargo.toml` (workspace root) | `[workspace.package] version` | `"0.11.0"` | `"0.11.0-ext.1"` |
| `plugins/org.eclipse.elk.js/package.json` | `"version"` | `"0.11.0"` | `"0.11.0-ext.1"` |
| `CHANGELOG.md` | Change history per ELK version | `## [0.11.0]` | `## [0.11.0-ext.1]` |

### Cargo Workspace Version Unification

Root `Cargo.toml` uses `workspace.package.version` so all crates inherit from a single source:

```toml
# Cargo.toml (root)
[workspace.package]
version = "0.11.0"
license = "EPL-2.0"
edition = "2021"

# Each crate's Cargo.toml
[package]
name = "org-eclipse-elk-core"
version.workspace = true
```

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
├─ releases/0.11.0          <- 0.11.0 stable release branch (tagged, published)
│   ├─ v0.11.0              <- release tag
│   ├─ v0.11.0+rs.1         <- hotfix tag
│   └─ v0.11.0+rs.2         <- hotfix tag
├─ custom/0.11.0            <- extension integration branch (main + custom features)
│   ├─ v0.11.0-ext.1        <- extension release tag
│   └─ v0.11.0-ext.2        <- extension release tag
├─ custom/{feature-name}    <- individual custom feature branches
├─ releases/0.12.0          <- next release branch (future)
├─ port/0.12.0              <- ELK 0.12.0 porting work branch
├─ feature/*                <- elk-rs feature development (NAPI, optimizations, etc.)
└─ fix/*                    <- bug fixes
```

**Rules:**
- `main` always passes build/test/parity — development happens here
- `releases/X.Y.Z` branches are created from `main` when ready to release
  - Release tags (`vX.Y.Z`) are placed on release branches
  - Hotfixes go to the release branch, then cherry-pick to `main` if needed
  - Release branches are long-lived (not deleted after release)
- `custom/X.Y.Z` branches integrate custom features on top of `main`
  - Extension tags (`vX.Y.Z-ext.N`) are placed on custom integration branches
  - Individual features are developed on `custom/{feature-name}` branches, merged into `custom/X.Y.Z`
  - See `CUSTOM_FEATURES.md` for feature documentation
- New ELK version porting happens on `port/X.Y.Z` branches, merged to `main`
- Feature/fix branches fork from `main`, merge via PR

### Tag Strategy

```
releases/0.11.0:
  v0.11.0                 <- stable release (ELK 0.11.0 parity, npm publish)
  v0.11.0+rs.1            <- elk-rs hotfix (npm publish if critical)
  v0.11.0+rs.2            <- elk-rs hotfix (npm publish if critical)

custom/0.11.0:
  v0.11.0-ext.1           <- extension release (custom features, npm/cargo publish)
  v0.11.0-ext.2           <- extension release (custom features, npm/cargo publish)

releases/0.12.0:
  v0.12.0                 <- stable release (ELK 0.12.0 parity, npm publish)
```

**Rules:**
- Stable release tags: `v{ELK_VERSION}` — matches ELK version, accompanies npm publish
- elk-rs hotfix tags: `v{ELK_VERSION}+rs.{N}` — on the release branch
- Extension tags: `v{ELK_VERSION}-ext.{N}` — on the `custom/X.Y.Z` integration branch
- Use annotated tags: `git tag -a v0.11.0 -m "elk-rs 0.11.0 (ELK 0.11.0 parity)"`
- Stable release tags are only placed on `releases/X.Y.Z` branches
- `+rs.N` tags are placed on the corresponding releases branch after hotfix commits
- `-ext.N` tags are placed on the corresponding `custom/X.Y.Z` branch after feature integration

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

# (Optional) Release-specific adjustments (README badges, final checks)
# git commit -m "release: elk-rs 0.12.0 final adjustments"

# Tag on the release branch
git tag -a v0.12.0 -m "elk-rs 0.12.0 (ELK 0.12.0 parity)"

# npm publish
cd plugins/org.eclipse.elk.js && sh build.sh && npm publish

# Push release branch and tags
git push origin releases/0.12.0 --tags

# Return to main for continued development
git checkout main
```

### Hotfix on Release Branch

```sh
# Work on the release branch
git checkout releases/0.11.0

# Apply fix, commit
git commit -m "fix: <description>"
git tag -a v0.11.0+rs.1 -m "elk-rs 0.11.0+rs.1 hotfix"

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
