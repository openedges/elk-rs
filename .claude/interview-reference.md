# Interview Reference

Conditional and lookup sections for the interview process. Core methodology (Definition Priority, Research, Critical Probes, Interview Style) is in `interview-protocol.md`.

Read the relevant section when needed — not all sections are required for every interview.

---

## Synthesis Checkpoints

At natural mid-points in the interview (roughly every 3-4 rounds for longer interviews), pause to synthesize.

### What to Include

```
중간 정리 (Round N):
- 지금까지 결정된 사항: ...
- 아직 미결인 사항: ...
- 발견된 긴장/모순: ...
- [Work Plan 참고] (if applicable): 추가 phase가 필요할 수 있는 발견 사항
```

### Confidence Indicators

When providing analysis, indicate confidence:
- **High** (multiple sources, established pattern): "This is well-established"
- **Medium** (some evidence, reasonable inference): "Based on available evidence..."
- **Low** (limited data, emerging area): "Informed guess — recommend validating"
- **Speculative** (no evidence, logical reasoning): "Speculating here..."

### Work Plan Notes at Checkpoints

If a checkpoint reveals potential work plan changes (new _shared impact, scope change, promotion candidate):
- **Note it verbally** in the synthesis. Do NOT modify the work plan mid-interview.
- The "Update Work Plan" step at interview completion will synthesize all checkpoint notes.

---

## Feature Map Update

When an interview results in changes to spec structure or definitions, update `feature-map.md` to reflect the current state. Feature-map must always mirror the actual specs — not plans, not intentions.

**Principle**: The skill that changes specs owns the feature-map update. Intake reads feature-map but never writes it.

**Triggers by skill:**

| Skill | Feature-map update |
|-------|-------------------|
| spec-intake | **Read only** — uses feature-map for classification |
| feature-create | Add new feature entry (Purpose, Boundary, Sub-features, Key interactions) |
| feature-modify | Update affected feature entry (Purpose, Boundary changes) |
| sub-feature-create | Update parent feature's Sub-features list + Boundary if clarified |
| sub-feature-modify | Update parent feature's Sub-features + Boundary. On promotion: add new feature entry + update original feature |
| shared-create | Add _shared entry + update affected features' Boundary and Key interactions |
| shared-modify | Update _shared entry + update affected features' Boundary and Key interactions |
| re-eval | **No feature-map update** — re-eval only modifies verification-criteria, not spec structure |
| reverse-spec | Create initial feature-map (code-analysis-based structure, user confirms) |
| reverse-spec-review | **No feature-map update** — reviews content, does not change spec structure |

---

## Spec State Management

Each spec.md has a **Status** field at the top indicating its current state.

### States

| State | Meaning |
|-------|---------|
| `In Progress` | Interview in progress or suspended with deferred items |
| `Extracted` | Reverse-engineered from code; pending review |
| `Confirmed` | Interview completed, spec confirmed |

### Ownership

Interview skills and reverse-spec skills manage spec state:
- **Interview start**: Set to `In Progress`
- **Interview complete**: Set to `Confirmed`
- **Deferred suspension**: Remains `In Progress`
- **Reverse-spec extraction**: Set to `Extracted`
- **Reverse-spec-review completion**: Transition `Extracted` → `Confirmed` (direct, no intermediate `In Progress`)

---

## Propose-Verify Mode

An alternative interaction mode for reviewing existing content rather than eliciting new content from scratch. Used when a spec already has content (extracted, imported, or previously written) and the goal is verification and correction, not generation.

### When to Use

When the section being reviewed already has content. The reviewer presents the existing content with its source context and asks for verification. If a section is empty (no content to propose), switch to standard interview mode for that section.

### Mechanism

Present content as a review request, not a confirmation request. Always state the content source:
- "코드 분석에서 직접 추출한 내용입니다" (high confidence, no tag)
- "코드 패턴에서 추론한 내용입니다 ([추론])" (inferred)

Frame the question as "이 내용에서 틀리거나 빠진 부분이 있습니까?" — not "맞습니까?"

### Anchoring Bias Mitigation

Propose-Verify carries inherent anchoring risk — users may accept AI-generated content uncritically because it appears plausible. Counter this by: (a) always stating the source and confidence level, (b) applying critical probes with the same rigor as standard interview mode, and (c) framing questions to invite correction rather than agreement.

### Critical Probes

At least 1 critical probe per round in Propose-Verify mode, same as standard interview. Category rotation still applies. Probes are particularly important here because the default user behavior is acceptance, not challenge.

---

## Deferred Decision Handling

When a user cannot answer a question during an interview because external confirmation or stakeholder alignment is needed, the system allows deferral rather than forcing an uncertain answer.

### When to Allow Deferral

- User explicitly states they need external input (domain expert, other team, business decision)
- User is uncertain and the answer requires information not available in the session

### Recording

Mark deferred items inline in spec.md at the relevant section:

```markdown
[DEFERRED] {question or decision needed}
- 이유: {why this can't be answered now}
```

Additional context (required materials, responsible person) is optional — include when the user provides it.

### Phase Status

When a phase has deferred items, update the work plan phase status:

```markdown
- **상태**: [deferred] N건 미결
```

Blocking judgment is at the phase level: if this phase's deferred items affect subsequent phases' core definitions, note it as blocking in the work plan.

### Re-entry

When deferred items are resolved and the user returns:
1. Re-invoke the **same interview skill** (a suspended create is still a create)
2. Read existing spec.md — Status (`In Progress`), written sections, and `[DEFERRED]` markers provide the resumption context
3. Address deferred items and continue with any sections not yet reached (inferred from Definition Priority order)
4. On completion, update Status to `Confirmed`
