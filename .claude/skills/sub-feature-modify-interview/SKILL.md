# Sub-feature Modify Interview

Modify an **existing** sub-feature's spec through structured interview. Checks consistency with parent feature and sibling sub-features.

## Trigger

After spec-intake classifies a requirement as modifying an existing sub-feature and user confirms. May be part of a work plan — check `specs/_work-plans/` for context.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-sub-feature-modify.md` — Known sub-feature modification failure patterns

Read when needed:
- `../../spec-levels.md` § Sub-feature — spec 수정 시 (Step 7)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

**Critical context — read before interviewing:**
- **The existing sub-feature spec.md** — current state
- **The existing verification-criteria.md** — current verification baseline
- **Parent feature's spec.md** — understand feature-level policies and cross-sub-feature stories
- **Sibling sub-features** — understand boundaries and avoid overlap
- **Referenced _shared specs** — understand shared data constraints
- The work plan if applicable

## Process

### 1. Understand Current State

Read and summarize the existing sub-feature spec to the user. Confirm understanding.

### 2. Define the Change

Focus on the delta:
- What policies are changing?
- Are stories being added/modified/removed?
- Are edge cases being added?
- **Why** is this change needed?

### 3. Parent Consistency Check

Verify the change doesn't conflict with parent feature:
- Does this change violate any feature-level policies?
- Do cross-sub-feature stories in the parent spec need updating?
- Does the parent's verification-criteria need re-evaluation?

### 4. Sibling Impact Check

For sibling sub-features:
- Does this change affect shared boundaries?
- Are there interactions that change?

### 5. Promotion Check

Watch for signs the sub-feature is outgrowing its level:
- Is the modification adding significant new capability areas?
- Is the sub-feature becoming too broad?
- If detected, verify purpose independence: "이 변경 후에도 이 sub-feature는 부모 feature의 일부인가, 아니면 독립적 기능으로 보는 게 맞나?"
- If purpose is independent, suggest promoting to a feature.

### 6. Update Verification Criteria

- Add/modify criteria for changed elements
- Check existing criteria are still valid

### 7. Generate Updated Spec

Produce updated spec.md. Present diff to user for review before writing.

### 8. Update Work Plan

If this interview was part of a work plan, mark the corresponding phase as completed (`[x]`) in the work plan document.

### 9. Reflection

Follow `../../reflection-protocol.md`.

Focus on:
- Did the change conflict with parent policies I didn't catch initially? (external signal)
- Did sibling sub-features need adjusting? (either source)
- Was a promotion signal detected? (self-judgment — note for intake gotchas too)

### 10. Re-evaluation Trigger

Based on the changes made in this interview, assess whether other specs' verification-criteria may need re-evaluation:

- Review the parent consistency check and sibling impact check from earlier steps
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

- **Start from the existing spec.** Show what's changing.
- Always **check upward**: "이 변경이 부모 feature의 정책과 충돌하지 않나요?"
- Always **check sideways**: "형제 sub-feature와의 경계가 바뀌나요?"
- Go deep on the change: exact new validation rules, exact new error handling, exact new states.

## Output

- Updated `specs/{feature-name}/{sub-feature-name}/spec.md`
- Updated `specs/{feature-name}/{sub-feature-name}/verification-criteria.md`
- Parent/sibling impact summary
- Updated `gotchas-sub-feature-modify.md` (via reflection, user confirmed)
