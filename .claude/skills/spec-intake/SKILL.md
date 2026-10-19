# Spec Intake

Classify incoming requirements and generate a work plan. This is the entry point for all spec work.

## Trigger

When the user describes a new requirement, feature request, change, or asks to work on spec.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `./feature-map.md` — Current project's feature boundaries (project-specific)
- `./gotchas-intake.md` — Known classification failure patterns

Read when needed:
- `../../spec-levels.md` — greenfield 시 level 정의 참조 (Step 2)

## Process

### 1. Understand the Input

Listen to the user's description. Ask clarifying questions if the input is too vague to classify. Do not ask more than 2-3 clarifying questions — move to classification with what you have and let the user correct.

### 2. Read Context

- Read `feature-map.md` to understand current feature landscape (Purpose, Boundary, Sub-features, Key interactions)
- Scan existing `specs/` folders to see what specs already exist
- Read `gotchas-intake.md` to avoid known classification mistakes
- If feature-map has no entries and specs/ is empty (greenfield), read `../../spec-levels.md` (level definitions) and `../../../examples/feature-map-example.md` (decomposition reference) to inform initial feature decomposition in the next step
- If `specs/_domain/` exists, read domain reference files. If the user's requirement references domain concepts not covered in domain reference, suggest running domain-knowledge-interview first

### 3. Classify

If greenfield (feature-map empty): propose an initial feature decomposition based on the user's input and `spec-levels.md` level definitions. Present the proposed features for user confirmation before generating the work plan. After confirmation, create initial feature-map entries.

If features exist: compare the user's input against each feature in feature-map.md.

Determine all affected specs:

**For each affected spec, determine:**
- Level: _shared / feature / sub-feature
- Action: create (new) / modify (existing)
- This maps to one of 6 interview skills:

| Level | Create | Modify |
|-------|--------|--------|
| _shared | shared-create-interview | shared-modify-interview |
| feature | feature-create-interview | feature-modify-interview |
| sub-feature | sub-feature-create-interview | sub-feature-modify-interview |

**_shared impact check:**
- Does this requirement involve data/entities used by 2+ features?
- Does feature-map.md's Key Interactions suggest cross-feature impact?
- Apply gotchas-intake.md patterns

### 4. Generate Work Plan

If the requirement affects multiple specs (common case), generate a work plan document at `specs/_work-plans/{date}-{title}.md`.

Work plan format:

```markdown
# Work Plan: {title}

**생성일**: {date}
**요구사항 요약**: {user's requirement in 1-2 sentences}

## 수정 범위 요약
- **영향받는 spec**: {count}개 ({spec paths, comma-separated})
- **_shared 변경**: {있음/없음}
- **신규 생성**: {count}개
- **기존 수정**: {count}개

## Phases

### Phase 1: {target spec path}
- **작업**: {create | modify}
- **스킬**: {skill name}
- **설명**: {what needs to be done and why this phase comes first}
- **상태**: [ ] 미완료 | [x] 완료 | [deferred] N건 미결

### Phase 2: {target spec path}
- **작업**: {create | modify}
- **스킬**: {skill name}
- **설명**: {what needs to be done}
- **의존성**: Phase 1 완료 후 진행
- **상태**: [ ] 미완료 | [x] 완료 | [deferred] N건 미결

### Phase 3: ...

## 영향 분석
- **_shared 변경**: {있음/없음 — 설명}
- **영향받는 feature**: {list}
```

**Phase 순서 원칙:**
1. _shared 변경이 있으면 항상 먼저 (다른 spec이 참조하므로)
2. Feature 변경 → sub-feature 변경 순서 (상위가 먼저)
3. Create → modify 순서 (새 spec이 있어야 기존 spec이 참조 가능)

If the requirement affects only a single spec, work plan is optional — direct hand-off is sufficient.

### 5. Present to User

Present the work plan (or single classification) to the user:

```
분류 결과:
- 영향받는 spec: {count}개
- _shared 변경: {있음/없음}
- Work plan: {path or "단일 작업 — work plan 불필요"}

[Work plan 내용 또는 단일 분류 결과]

이 계획으로 진행할까요?
```

Ask the user to confirm, modify, or redirect.

### 5a. Handle Rejection/Modification

If the user rejects or modifies the classification:

1. **Present your reasoning**: Explain why you classified this way — which feature-map entries matched, what gotchas influenced the decision, why you flagged (or didn't flag) _shared impact.
2. **Listen to the correction**: Understand what the user sees differently and why.
3. **Re-classify**: Incorporate the user's feedback and regenerate the work plan.
4. **Re-confirm**: Present the updated plan for approval. Repeat until confirmed.

This correction is **high-value feedback for reflection**. Record:
- What was the original classification and reasoning
- What was the user's correction and reasoning
- This becomes a gotchas candidate (e.g., "address-related requests classified as graph-design but user redirected to rfi-system because...")

### 6. Hand Off

After user confirmation:
- If work plan generated → guide user to start Phase 1 by invoking the specified skill with the work plan path as reference
- If single classification → suggest invoking the appropriate interview skill directly
- User invokes each skill manually, referencing the work plan for context

### 7. Reflection

Follow the protocol in `../../reflection-protocol.md`.

Focus on classification accuracy:
- Did the user redirect my classification? (external signal)
- Was I uncertain between multiple features? (self-judgment)
- Did I miss a _shared impact? (either source)
- Did the work plan need reordering? (external signal)

## Output

1. Work plan document at `specs/_work-plans/{date}-{title}.md` (if multi-spec)
2. Classification decision (confirmed by user)
3. Updated feature-map.md (if new feature)
4. Gotchas updates (via reflection, confirmed by user)

## Gotchas

Read `./gotchas-intake.md` at the start of every execution.
