# Feature Create Interview

Define a **new** feature's spec through structured interview. Produces spec.md + verification-criteria.md for a feature-level folder.

## Trigger

After spec-intake classifies a requirement as a new feature (create) and user confirms. May be part of a work plan — check `specs/_work-plans/` for context.

**Prerequisite**: A work plan or intake classification must exist. If the user invokes this skill directly without prior intake, guide them to run spec-intake first. All interviews require intake as the entry point.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `./gotchas-feature.md` — Known feature-interview failure patterns

Read when needed:
- `../../spec-levels.md` § Feature — spec 생성 시 (Step 9)
- `../../interview-reference.md` § Feature Map Update — 인터뷰 완료 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

Also read for context:
- The project's `feature-map.md` (in spec-intake skill folder) — understand the feature landscape
- Any `_shared/` specs referenced by this feature
- Adjacent features referenced in the classification

## Process

### 1. Establish Purpose

Start with the core question: "이 기능이 왜 존재하는가?"

The answer should be a clear, standalone statement. If the user can't answer this independently of other features, it might be a sub-feature — flag this.

### 2. Define Sub-features

Ask the user to list the capabilities this feature will have. For each sub-feature, capture:
- Name
- Brief role (1-2 sentences)

Do not deep-dive into sub-feature details — that's for sub-feature-interview.

### 3. Define Policies

Identify feature-level business rules. Focus on:
- Cross-sub-feature policies (how sub-features interact)
- Constraints and invariants at the feature level
- What is NOT allowed

Apply gotchas-feature.md patterns — common policy gaps to probe for.

### 4. Define Cross-sub-feature Stories

Capture user flows that span multiple sub-features:
- What is the main happy path across sub-features?
- What are the interaction policies? (sync timing, conflict resolution, data flow)

### 5. Define Output

What does this feature produce? What data or state changes?

### 6. Define References

Identify dependencies:
- Which `_shared/` domains does this feature reference?
- Which other features does it interact with?
- Are there new _shared candidates? (data that will likely be shared)

### 7. Define Edge Cases

Probe for feature-level edge cases. Use gotchas patterns to ask about commonly missed scenarios.

### 8. Generate Verification Criteria

Based on the interview, generate verification-criteria.md with P0/P1/P2 items.
Focus on feature-level scope: cross-sub-feature criteria, integration criteria, completeness.

### 9. Generate Spec

Create the feature folder with:
- `spec.md` — following the template in `../../spec-levels.md` (Feature section)
- `verification-criteria.md` — P0/P1/P2 checklist

Present both to the user for review before writing files.

### 10. Update Work Plan

If this interview was part of a work plan, mark the corresponding phase as completed (`[x]`) in the work plan document.

### 11. Reflection

Follow `../../reflection-protocol.md`.

Focus on interview quality:
- Did the user have to correct my understanding? (external signal)
- Were there policies I didn't ask about that the user added? (external signal)
- Were there sub-features I suggested that the user rejected? (self-judgment)

### 12. Re-evaluation Trigger

Assess whether this new feature affects verification-criteria of existing specs:

- Does this new feature reference existing _shared or feature specs? If so, do those specs' verification-criteria need to account for this new reference?
- Will existing specs reference this new feature? If so, do their verification-criteria need updating?
- If any affected specs are identified, report to the user:

```
영향받는 verification-criteria:
- {spec path 1} — {새 feature가 영향을 미치는 이유}
- {spec path 2} — {새 feature가 영향을 미치는 이유}

re-eval-interview 실행을 권장합니다.
```

Do NOT add phases to the work plan. The re-eval-interview skill manages its own work plan recording.

## Interview Style

- Ask 2-4 questions per round. Don't overwhelm.
- Challenge assumptions: "이 정책에 예외는 없나요?", "이 sub-feature 없이 feature가 동작할 수 있나요?"
- Use concrete examples from the user's domain when probing.
- If the user is unsure, propose options with trade-offs rather than waiting.

## Output

- `specs/{feature-name}/spec.md`
- `specs/{feature-name}/verification-criteria.md`
- Updated `gotchas-feature.md` (via reflection, user confirmed)
