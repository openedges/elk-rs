# Spec Level Details

Level-specific definitions for each spec level. Core concepts, verification criteria, and key rules are in `spec-structure.md`.

---

## Reference Layer: _domain

### Role

Organizes domain knowledge from external sources (documents, data sheets, domain expert input) into structured reference files. Records domain facts — not service design decisions.

### What belongs in _domain

- Domain concepts and terminology
- Relationships and dependencies between domain concepts
- Domain constraints and rules (compatibility matrices, valid combinations, physical limits)
- Reference data tables and values

### What does NOT belong in _domain

- Service-level design decisions (limits, policies chosen by the team) — those go in `_shared`
- Spec structure (no spec.md / verification-criteria.md) — _domain uses its own file format
- Implementation details

### File format

Each file covers one coherent domain area:

```markdown
# {Domain Area Title}

**Status**: Current
**Last updated**: {date}
**Source**: {document name, version, or description}

{organized content — structured for readability and reference}
```

Unverified sections are marked inline:

```markdown
[UNVERIFIED] {reason}
- value 1
- value 2
```

### Managed by

`domain-knowledge-interview` skill. See that skill for the full structuring process.

---

## Pre-spec: _ideas

### Role

Stores unvalidated ideas from brainstorming sessions. Ideas are pre-spec artifacts — they may eventually feed into intake as material for spec creation, but they are not specs themselves.

### What belongs in _ideas

- Service-related ideas at any maturity level
- Background context, open questions, preliminary thoughts
- References to existing specs and domain knowledge

### What does NOT belong in _ideas

- Validated requirements (those go through intake → interview)
- Domain facts from external sources (those go in `_domain`)

### File format

Each file covers one idea. Format is defined by the brainstorming skill — lighter than spec.md, no spec.md/verification-criteria.md structure. Status field: `Draft` (exploring) or `Developed` (ready to act upon).

### Managed by

`brainstorming` skill. Does NOT follow the Uniform Structure Principle.

---

## Level 0: _shared

### Role

Defines service-level data models, entities, and policies that are referenced by 2 or more features. Serves as the "shared kernel" across feature boundaries. Design decisions here are informed by `_domain` facts but owned by the team — rationale is required.

### When to create

- **Default: put it in a feature first.** When a second feature needs the same concept, extract it to `_shared`.
- **Exception:** If the spec-intake skill determines with high confidence that a concept is inherently shared (e.g., a core data model that multiple features will obviously reference), it may be placed in `_shared` from the start.

### What belongs in _shared

- Shared data entities and their relationships
- Cross-feature constraints (e.g., "entity X can only exist if entity Y exists")
- Shared enums, status definitions, and terminology
- Cross-feature stories (user flows that span multiple features)

### What does NOT belong in _shared

- Anything used by only one feature — keep it in that feature
- UI-specific definitions — those belong in the feature or sub-feature
- Implementation details (database schema, API design) — those belong in dev-side docs

### spec.md structure

```markdown
# {Shared Domain Name}

## Purpose
Why this shared domain exists. What it provides to features.

## Entities
- Entity definitions, relationships, constraints

## Policies
Cross-feature business rules owned by this shared domain.
(e.g., "Node must have at least 1 Port")

## Cross-feature Stories
### {Story Name}
Flows that span multiple features, described here because
no single feature owns them.

## Edge Cases
Cross-feature edge cases and how to handle them.

## References
- `_domain/{name}` — domain facts this shared domain's design decisions are based on
- `feature/{name}` — features that reference this shared domain
```

---

## Level 1: Feature

### Role

A cohesive unit of functionality from the user's perspective. Has its own purpose (why it exists), policies (business rules), and sub-features. Roughly corresponds to a bounded context in DDD terms. Exists as a **top-level folder** under `specs/`.

### Size guideline

A feature is defined by having an **independent purpose**. If you can answer "why does this exist?" with a clear, standalone statement, it's a feature. If the answer is "because feature X needs it," it's probably a sub-feature of X.

### What goes in feature spec.md

- Purpose: why this feature exists
- Sub-feature list and overview
- Cross-sub-feature stories (user flows that span multiple sub-features within this feature)
- Interaction policies between sub-features (e.g., "table-view and graph-view sync in real-time")
- Connections to other features (how this feature relates to others)
- Overall output/behavior definition

### What does NOT go in feature spec.md

- Sub-feature-specific policies — those go in the sub-feature's own spec.md
- Implementation details
- Detailed UI specifications (unless they define a policy)

### What goes in feature verification-criteria.md

- Cross-sub-feature criteria (e.g., "sync between table-view and graph-view is defined")
- Feature-level completeness criteria (e.g., "all sub-features are listed")
- Integration criteria with other features

### spec.md structure

```markdown
# {Feature Name}

## Purpose
Why this feature exists. What user need it addresses.

## Sub-features
Brief overview of each sub-feature and its role.

## Policies
Feature-level business rules (cross-sub-feature scope).

## Cross-sub-feature Stories
### {Story Name}
User flows that span multiple sub-features.
Include interaction policies (sync timing, conflict resolution, etc.)

## Edge Cases
Feature-level edge cases (cross-sub-feature scope).

## Output
What this feature produces. What data/state changes when this feature is used.

## References
- `_shared/{name}` — shared domains this feature references
- `feature/{name}` — other features this feature interacts with
```

---

## Level 2: Sub-feature

### Role

A specific capability within a feature. Has its own policies, stories, and verification criteria. Exists as a **nested folder** under a feature.

### Size guideline

A sub-feature is a capability that serves its parent feature's purpose. It should be focused enough to be independently describable, but not so large that it needs its own sub-features.

### Promotion rule

A sub-feature should be **promoted to a feature** when its purpose becomes independent of the parent feature — when it can answer "why does this exist?" without referencing the parent.

Size signals (growing complexity, multiple capability areas, sub-sub-features emerging) are **detection triggers**, not promotion criteria. They indicate that independence may have emerged, but the actual test is purpose independence.

Verification: apply feature-create Step 1 — "이 기능이 왜 존재하는가?" must have a clear, standalone answer without mentioning the parent.

Promotion is a simple move: relocate the folder from `specs/{feature}/{sub-feature}/` to `specs/{sub-feature}/`. The internal structure (spec.md + verification-criteria.md) stays identical.

### What goes in sub-feature spec.md

- Purpose within the parent feature
- Policies specific to this sub-feature (e.g., input validation rules, display rules)
- Stories: concrete user flows within this sub-feature
- Edge cases and how to handle them

### What does NOT go in sub-feature spec.md

- Cross-sub-feature interaction policies — those go in the parent feature's spec.md
- Shared data definitions — those go in _shared

### What goes in sub-feature verification-criteria.md

- Sub-feature-specific criteria (e.g., "cell input validation rules are defined")
- Story coverage criteria (e.g., "error handling flow is defined")

### spec.md structure

```markdown
# {Sub-feature Name}

## Purpose
What this sub-feature does within the parent feature.

## Policies
Business rules specific to this sub-feature.

## Stories
### {Story Name}
Concrete user flow: given X, when Y, then Z.

### {Story Name}
Another flow...

## Edge Cases
Known edge cases and how to handle them.
```
