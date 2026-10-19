# Spec Structure Definition

This document defines the core structure for organizing spec documents. It is tool-agnostic and project-agnostic — any project can adopt this structure by creating a `specs/` directory and a project-specific `feature-map.md`.

Level-specific details (_domain, _ideas, _shared, feature, sub-feature) are in `spec-levels.md`.

---

## Core Concepts

### Layers

The spec system has two distinct layers: a **reference layer** and a **spec layer**.

**Reference layer** (`_domain`): Organized domain knowledge from external sources. Records "what is true in the domain." Not a design decision — serves as input to spec work.

**Spec layer** (`_shared`, `feature`, `sub-feature`): Service-level design decisions. Records "how we model the domain in our service." Requires rationale. The spec layer has 3 levels, each with the same internal structure (spec.md + verification-criteria.md).

| Layer | Level | Name | Form | Contains |
|-------|-------|------|------|----------|
| Reference | — | **_domain** | Folder | Domain facts, terminology, constraints, reference data (from external sources) |
| Pre-spec | — | **_ideas** | Folder | Brainstorming artifacts, unvalidated ideas (from brainstorming skill) |
| Spec | 0 | **_shared** | Folder | Service-level data models, entities, and policies referenced by 2+ features |
| Spec | 1 | **feature** | Folder (top-level under specs/) | A cohesive unit of functionality with its own purpose |
| Spec | 2 | **sub-feature** | Folder (nested under a feature) | A specific capability within a feature |

### Relationship: _domain → _shared

_domain and _shared are distinct layers with different roles:

- **_domain** records domain facts — e.g., "An order can contain 1-N items." Source-tracked, not owned by the team.
- **_shared** records service design decisions informed by domain facts — e.g., "Our system limits items per order to 50." Rationale required, owned by the team.

_shared specs should reference `_domain` files when design decisions are based on domain facts. This enables traceability: when a domain fact changes, affected _shared specs can be identified through references.

### Folder Structure

```
specs/
├── _domain/
│   └── {domain-area}.md             # Domain reference (no spec.md/v-criteria structure)
│
├── _ideas/
│   └── {idea-title}.md              # Idea artifact (own format, not spec.md)
│
├── _shared/
│   └── {shared-domain}/
│       ├── spec.md
│       └── verification-criteria.md
│
├── {feature-name}/
│   ├── spec.md                      # Feature purpose + cross-sub-feature stories
│   ├── verification-criteria.md     # Feature-level criteria (cross-sub-feature)
│   │
│   ├── {sub-feature-a}/
│   │   ├── spec.md                  # Sub-feature policies + stories
│   │   └── verification-criteria.md # Sub-feature-specific criteria
│   │
│   └── {sub-feature-b}/
│       ├── spec.md
│       └── verification-criteria.md
│
└── _meta/
    └── glossary.md                  # (optional) shared terminology
```

### Uniform Structure Principle

Features and sub-features share the same internal structure: `spec.md` + `verification-criteria.md`. This uniformity enables:
- **Seamless promotion**: Moving a sub-feature folder to `specs/` top-level makes it a feature. No restructuring needed.
- **Consistent tooling**: Skills operate the same way regardless of level.
- **Predictable navigation**: Claude and humans always know what to expect inside any folder.

---

## Verification Criteria

Every folder (_shared, feature, and sub-feature) contains its own `verification-criteria.md`. Each file covers only the criteria relevant to that level.

| Level | Criteria scope |
|-------|---------------|
| **_shared** | Entity completeness, cross-feature policy coverage, constraint consistency |
| **Feature** | Cross-sub-feature criteria, integration criteria, feature-level completeness |
| **Sub-feature** | Sub-feature-specific policies, story coverage, edge case coverage |

### Purpose

A prioritized checklist of what must be true for this spec to be considered complete. Used for:
- Spec review (is the spec complete?)
- Cross-check on spec modification (git diff against previous version)
- Guiding dev-side test coverage

### Priority levels

| Level | Meaning | Action |
|-------|---------|--------|
| **P0** | Must be defined. Spec is incomplete without this. | Block review until resolved |
| **P1** | Should be defined. Important but not blocking. | Flag during review |
| **P2** | Nice to have. Can be deferred. | Track as open item |

### File structure

```markdown
# Verification Criteria: {Name}

## P0 (Must)
- [ ] {Criterion}
- [ ] {Criterion}

## P1 (Should)
- [ ] {Criterion}

## P2 (Nice to have)
- [ ] {Criterion}
```

---

## References Convention

Every spec.md includes a `## References` section listing dependencies on other specs.

### Format

```markdown
## References
- `_domain/{name}` — {domain fact this design decision is based on}
- `_shared/{name}` — {brief description of what is referenced}
- `feature/{name}` — {brief description of the interaction}
- `feature/{name}/{sub-feature}` — {if referencing a specific sub-feature}
```

### Rules

- Use backtick-wrapped relative paths (parseable via regex for future automation)
- Each reference includes a brief description of WHY this spec depends on the other
- When a referenced spec changes, the referencing spec's verification-criteria should be re-evaluated — each interview skill reports affected specs at the end (Re-evaluation Trigger step), and the user invokes re-eval-interview to process them
- When a `_domain` reference is updated, `_shared` specs that reference it should also be re-evaluated — domain-knowledge-interview reports affected `_shared` specs at the end

---

## Key Rules Summary

1. **_domain is reference, _shared is design.** _domain records domain facts from external sources. _shared records service-level design decisions informed by those facts. Do not mix them.
2. **All spec levels are folders with the same internal structure** (spec.md + verification-criteria.md). Features are top-level, sub-features are nested. _domain uses its own file format (no spec.md/v-criteria).
3. **_shared is for 2+ feature references.** Default: feature first, extract when needed.
4. **Stories live at the level that owns them.** Cross-sub-feature stories → feature spec.md. Cross-feature stories → _shared spec.md. Sub-feature stories → sub-feature spec.md.
5. **Promote when it outgrows its level.** Sub-feature needs its own sub-folders → move it to top-level as a feature. No restructuring needed.
6. **Verification criteria are scoped to their level.** Feature criteria cover cross-sub-feature concerns. Sub-feature criteria cover their own scope.
7. **References are explicit.** Every cross-spec dependency is declared in the References section. _shared specs should reference `_domain` when design decisions are based on domain facts.
