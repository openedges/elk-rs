# Shared Create Interview

Define a **new** _shared domain's spec through structured interview. Produces spec.md + verification-criteria.md for a _shared folder.

_shared domains define data models, entities, and policies referenced by 2+ features. This interview focuses on **entities, relationships, and cross-feature constraints** — not user-facing functionality.

## Trigger

After spec-intake classifies a requirement as needing a new _shared domain and user confirms. May be part of a work plan — check `specs/_work-plans/` for context.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-shared-create.md` — Known _shared creation failure patterns

Read when needed:
- `../../spec-levels.md` § _shared — spec 생성 시 (Step 8)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

Also read for context:
- The project's `feature-map.md` (in spec-intake skill folder) — which features will reference this _shared domain
- Existing `_shared/` specs — avoid overlap with existing shared domains
- Feature specs that triggered this _shared creation — understand the extraction context
- `specs/_domain/` files — domain facts that inform entity and policy design decisions

## Process

### 1. Establish Purpose

Start with: "이 공유 도메인이 feature들에 무엇을 제공하는가?"

Clarify why this needs to be shared rather than staying in a single feature. If only one feature references it currently, confirm that a second feature is expected.

### 2. Define Entities

This is the core of _shared interviews (unlike feature interviews which focus on user flows):
- What are the data entities?
- How do they relate to each other?
- What are the constraints? (required fields, uniqueness, cardinality)
- What are the valid states and transitions?
- Cross-check with `_domain` reference: are entity definitions consistent with domain facts?
- When a design decision diverges from domain constraints, record the rationale and reference the relevant `_domain` file.

### 3. Define Cross-feature Policies

Identify policies that apply across all features referencing this domain:
- Invariants: "Entity X can only exist if Entity Y exists"
- Ownership rules: which feature can create/modify/delete these entities?
- Consistency rules: what must be true across features?

### 4. Define Cross-feature Stories

Capture user flows that span multiple features through this shared data:
- How does data flow from one feature to another via these entities?
- What happens when one feature modifies shared data that another feature depends on?

### 5. Define Edge Cases

Probe for cross-feature edge cases:
- Concurrent modification from different features
- Deletion cascades
- Invalid state combinations

### 6. Identify Referencing Features

List all features that will reference this _shared domain. For each:
- What part of the _shared domain do they use?
- Read/write access?
- This informs the References section and verification-criteria

### 7. Generate Verification Criteria

Focus on _shared-specific criteria:
- Entity completeness: are all entities and relationships defined?
- Cross-feature policy coverage: are all cross-feature constraints defined?
- Constraint consistency: do policies contradict each other?

### 8. Generate Spec

Create the _shared folder with:
- `spec.md` — following the template in `../../spec-levels.md` (_shared section)
- `verification-criteria.md` — P0/P1/P2 checklist

Present both to the user for review before writing files.

### 9. Update Work Plan

If this interview was part of a work plan, mark the corresponding phase as completed (`[x]`) in the work plan document.

### 10. Reflection

Follow `../../reflection-protocol.md`.

Focus on:
- Did I miss entities the user had to add? (external signal)
- Did I miss cross-feature constraints? (either source)
- Was the extraction from a feature clean, or did ambiguous boundaries remain? (self-judgment)

### 11. Re-evaluation Trigger

Assess whether this new _shared spec affects verification-criteria of existing specs:

- Does this new _shared spec introduce data/policies that existing features should account for in their verification-criteria?
- Will existing features reference this new _shared spec? If so, do their verification-criteria need updating?
- If any affected specs are identified, report to the user:

```
영향받는 verification-criteria:
- {spec path 1} — {새 _shared spec이 영향을 미치는 이유}
- {spec path 2} — {새 _shared spec이 영향을 미치는 이유}

re-eval-interview 실행을 권장합니다.
```

Do NOT add phases to the work plan. The re-eval-interview skill manages its own work plan recording.

## Interview Style

- Focus on **data modeling** more than user experience. Ask about entities, relationships, constraints.
- Challenge completeness: "이 엔티티에 다른 상태가 있을 수 있나요?", "이 관계가 1:N인지 N:M인지?"
- Probe cross-feature impact: "Feature A에서 이 데이터를 수정하면 Feature B에 어떤 영향?"
- Keep it abstract — avoid implementation details (DB schema, API). Focus on domain model.

## Output

- `specs/_shared/{domain-name}/spec.md`
- `specs/_shared/{domain-name}/verification-criteria.md`
- Updated `gotchas-shared-create.md` (via reflection, user confirmed)
