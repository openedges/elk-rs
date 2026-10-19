# Shared Modify Interview

Modify an **existing** _shared domain's spec through structured interview. This is the **highest-risk operation** in the spec system — changes here cascade to all referencing features.

## Trigger

After spec-intake classifies a requirement as modifying an existing _shared domain and user confirms. Almost always part of a work plan — _shared modify is typically Phase 1.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-shared-modify.md` — Known _shared modification failure patterns

Read when needed:
- `../../spec-levels.md` § _shared — spec 수정 시 (Step 6)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

**Critical context — read before interviewing:**
- **The existing _shared spec.md** — current state of the shared domain
- **The existing verification-criteria.md** — current verification baseline
- **ALL feature specs that reference this _shared domain** (check References sections) — understand cascade impact
- **`specs/_domain/` files** referenced by this _shared spec — verify that changes remain consistent with domain facts
- The work plan that triggered this modification — understand the broader context

## Process

### 1. Understand Current State

Read and summarize the existing _shared spec to the user. Confirm understanding before proceeding.

### 2. Define the Change

Focus on the delta:
- What entities are being added/modified/removed?
- What relationships change?
- What policies change?
- **Why** is this change needed? (Record the rationale)

### 3. Cascade Impact Analysis

**This is the most critical step.** For each feature that references this _shared domain:
- How does this change affect that feature's spec?
- Does the feature's verification-criteria need updating?
- Are there sub-features that will be impacted?
- Does this change break any existing stories?

Present the impact analysis to the user:

```
영향 분석:
- _shared/graph-data-model 변경: AddressMapRemap 엔티티 추가
- 영향받는 feature:
  1. graph-design — address-map sub-feature에서 remap 참조 필요
  2. rfi-system — version snapshot에 새 엔티티 포함 필요
- 영향받는 verification-criteria: 2개
- 추가 work plan phase 필요 여부: 있음
```

### 4. Define Edge Cases for the Change

Probe change-specific edge cases:
- What happens to existing data when this change is applied?
- Backward compatibility: do existing features still work with the changed model?
- Migration: is there a transition period?

### 5. Update Verification Criteria

Update the _shared verification-criteria.md:
- Add criteria for the new/changed elements
- Check existing criteria are still valid

### 6. Generate Updated Spec

Produce the updated spec.md showing:
- What changed (diff-friendly — clearly mark additions/modifications)
- Updated References section if new features are affected

Present to user for review before writing. **Show diff against previous version.**

### 7. Update Work Plan

If this interview was part of a work plan, mark the corresponding phase as completed (`[x]`) in the work plan document.

If the cascade analysis reveals additional changes in referencing features that require full spec modification, propose adding modify phases to the work plan (불가피한 경우의 후속 단계 수정).

### 8. Reflection

Follow `../../reflection-protocol.md`.

Focus on:
- Did I miss a referencing feature in the cascade analysis? (critical — external signal)
- Did the user add impact I didn't foresee? (external signal)
- Was the change scope larger than initially classified by intake? (self-judgment)

### 9. Re-evaluation Trigger

Based on the changes made in this interview, assess whether other specs' verification-criteria may need re-evaluation:

- Review the cascade impact analysis from earlier steps
- Identify specs whose verification-criteria may be affected by this change
- If any are identified, report to the user:

```
영향받는 verification-criteria:
- {spec path 1} — {변경이 영향을 미치는 이유}
- {spec path 2} — {변경이 영향을 미치는 이유}

re-eval-interview 실행을 권장합니다.
```

Do NOT add phases to the work plan. The re-eval-interview skill manages its own work plan recording.

## Interview Style

- **Start from the existing spec** — never ignore current state.
- Be **conservative**: probe for reasons NOT to change before accepting changes. "이 변경 없이 해결할 수 있는 방법은 없나요?"
- **Cascade-first thinking**: every question should consider "이게 다른 feature에 어떤 영향?"
- Present impact analysis visually — tables or lists, not paragraphs.

## Output

- Updated `specs/_shared/{domain-name}/spec.md`
- Updated `specs/_shared/{domain-name}/verification-criteria.md`
- Impact analysis (included in work plan or presented to user)
- Potential additional work plan phases
- Updated `gotchas-shared-modify.md` (via reflection, user confirmed)
