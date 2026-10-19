# Sub-feature Create Interview

Define a **new** sub-feature's spec through structured interview. Produces spec.md + verification-criteria.md for a sub-feature folder nested under a feature.

## Trigger

After spec-intake classifies a requirement as a new sub-feature (create) and user confirms. May be part of a work plan — check `specs/_work-plans/` for context.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-sub-feature.md` — Known sub-feature-interview failure patterns

Read when needed:
- `../../spec-levels.md` § Sub-feature — spec 생성 시 (Step 7)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

Also read for context:
- **Parent feature's spec.md** — understand the feature's purpose, policies, and how this sub-feature fits
- **Parent feature's verification-criteria.md** — understand what the feature expects from sub-features
- **Sibling sub-features** (if they exist) — avoid overlap, understand boundaries
- **Referenced _shared specs** — understand shared data this sub-feature will use

## Process

### 1. Establish Purpose within Parent

Start with: "이 sub-feature가 부모 feature 안에서 하는 역할은?"

The answer should connect to the parent feature's purpose. If it doesn't, it might belong to a different feature — flag this.

### 2. Define Policies

Identify sub-feature-specific business rules:
- Input validation rules
- Display/behavior rules
- Processing rules
- What is NOT allowed at this level

Important: Do NOT redefine parent feature's policies here. Only policies owned by this sub-feature.
Apply gotchas-sub-feature.md patterns.

### 3. Define Stories

Capture concrete user flows within this sub-feature:
- Happy path: given X, when Y, then Z
- Error paths: what happens when things go wrong
- Alternative paths: other ways users might interact

Each story should be specific enough to verify.

### 4. Define Edge Cases

Probe for sub-feature-specific edge cases:
- Boundary values
- Unexpected input
- Concurrent operations
- Empty/null states

### 5. Generate Verification Criteria

Based on the interview, generate verification-criteria.md:
- Policy coverage: is every policy verifiable?
- Story coverage: are all paths defined?
- Edge case coverage: are known edge cases addressed?

### 6. Check Promotion Signals

During the interview, watch for signs that this sub-feature is too large:
- More than 5-6 distinct stories
- Multiple distinct capability areas within it
- User describing what are essentially sub-sub-features

If detected, verify purpose independence: "이 sub-feature가 부모 feature 없이 독립적으로 존재 이유를 설명할 수 있나요?" If yes, suggest promoting to a feature. This is a classification correction — note it for reflection.

### 7. Generate Spec

Create the sub-feature folder with:
- `spec.md` — following the template in `../../spec-levels.md` (Sub-feature section)
- `verification-criteria.md` — P0/P1/P2 checklist

Present both to the user for review before writing files.

### 8. Update Work Plan

If this interview was part of a work plan, mark the corresponding phase as completed (`[x]`) in the work plan document.

### 9. Reflection

Follow `../../reflection-protocol.md`.

Focus on:
- Did the user add policies I didn't probe for? (external signal)
- Did I suggest stories the user rejected as out of scope? (self-judgment)
- Was a promotion signal detected? (self-judgment — important for intake gotchas too)

### 10. Re-evaluation Trigger

Assess whether this new sub-feature affects verification-criteria of existing specs:

- Does the parent feature's verification-criteria need to account for this new sub-feature?
- Do sibling sub-features' verification-criteria need updating due to boundary changes?
- If any affected specs are identified, report to the user:

```
영향받는 verification-criteria:
- {spec path 1} — {새 sub-feature가 영향을 미치는 이유}
- {spec path 2} — {새 sub-feature가 영향을 미치는 이유}

re-eval-interview 실행을 권장합니다.
```

Do NOT add phases to the work plan. The re-eval-interview skill manages its own work plan recording.

## Interview Style

- Go deeper than feature-interview. Ask about specifics: exact validation rules, exact error messages, exact state transitions.
- Challenge completeness: "이 상황에서는 어떻게 되나요?", "에러가 나면 사용자에게 뭘 보여주나요?"
- Reference parent feature's policies when probing: "부모 feature에서 X라고 했는데, 여기서는 어떻게 적용되나요?"
- Use concrete examples with real data when possible.

## Output

- `specs/{feature-name}/{sub-feature-name}/spec.md`
- `specs/{feature-name}/{sub-feature-name}/verification-criteria.md`
- Updated `gotchas-sub-feature.md` (via reflection, user confirmed)
