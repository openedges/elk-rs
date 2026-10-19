# Feature Modify Interview

Modify an **existing** feature's spec through structured interview. Assesses change impact on sub-features and referencing features.

## Trigger

After spec-intake classifies a requirement as modifying an existing feature and user confirms. May be part of a work plan — check `specs/_work-plans/` for context.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-feature-modify.md` — Known feature modification failure patterns

Read when needed:
- `../../spec-levels.md` § Feature — spec 수정 시 (Step 7)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

**Critical context — read before interviewing:**
- **The existing feature spec.md** — current state
- **The existing verification-criteria.md** — current verification baseline
- **All sub-feature specs** under this feature — understand what depends on feature-level policies
- **Referenced _shared specs** — understand shared data constraints
- **Other features that reference this feature** (check References sections)
- The work plan if applicable

## Process

### 1. Understand Current State

Read and summarize the existing feature spec to the user. Confirm understanding.

### 2. Define the Change

Focus on the delta:
- What policies are changing?
- Are sub-features being added/removed/reorganized?
- Are cross-sub-feature stories changing?
- Are references changing?
- **Why** is this change needed?

### 3. Sub-feature Impact Analysis

For each sub-feature under this feature:
- Does this change affect the sub-feature's scope or policies?
- Does the sub-feature's verification-criteria need updating?
- Are any sub-feature stories invalidated?

### 4. External Impact Analysis

For features/specs that reference this feature:
- Does this change affect them?
- Do their verification-criteria need re-evaluation?

### 5. Define Edge Cases for the Change

- Does this change conflict with existing sub-feature policies?
- Backward compatibility with existing stories?

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
- Did I miss a sub-feature impact? (external signal)
- Was the change scope larger than initially classified? (self-judgment)
- Did cross-sub-feature stories need updating that I didn't anticipate? (either source)

### 10. Re-evaluation Trigger

Based on the changes made in this interview, assess whether other specs' verification-criteria may need re-evaluation:

- Review the sub-feature and external impact analysis from earlier steps
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

- **Start from the existing spec.** Always show what's changing relative to current state.
- Challenge the change: "이 정책을 바꾸면 기존 sub-feature X는 어떻게 되나요?"
- Focus on **impact breadth** — feature changes ripple downward to sub-features.
- Present impact analysis as a checklist.

## Output

- Updated `specs/{feature-name}/spec.md`
- Updated `specs/{feature-name}/verification-criteria.md`
- Sub-feature impact summary
- Updated `gotchas-feature-modify.md` (via reflection, user confirmed)
