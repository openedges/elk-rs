# Re-eval Interview

Re-evaluate verification-criteria of specs affected by changes in other specs. Handles all levels (_shared, feature, sub-feature) in a single invocation.

## Why a Single Skill

One change can affect multiple specs across different levels simultaneously. Unlike create/modify interviews (which focus on a single target spec), re-evaluation is inherently **cross-level and multi-target**. A single skill sees the full impact picture — splitting by level would fragment visibility.

## Trigger

After any interview skill (create or modify) completes and reports specs needing re-evaluation. The user invokes this skill with the context of:
- What changed (which spec, what kind of change)
- Which specs are potentially affected (reported by the preceding interview skill)

**Prerequisite**: A preceding interview skill must have identified affected specs. If the user invokes this skill without context, ask what change triggered the need for re-evaluation.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions, reference rules
- `./gotchas-re-eval.md` — Known re-evaluation failure patterns

Read when needed:
- `../../spec-levels.md` § {해당 level} — level-specific 추가 체크 시 (Step 3)

**Critical context — read before re-evaluating:**
- **The changed spec** (trigger source) — understand what changed and why
- **All affected specs' verification-criteria.md** — current verification baseline for each
- **All affected specs' spec.md** — understand context for each criterion
- The work plan if applicable — understand broader context

## Process

### 1. Understand the Trigger Change

Read the spec that was just created or modified. Summarize to the user:
- What changed (entities, policies, stories, relationships)
- Which specs are reported as potentially affected
- Brief assessment of expected impact severity per affected spec

### 2. Review Each Affected Spec's Verification Criteria

For each affected spec, review every criterion in its verification-criteria.md:

**Categorize each criterion:**
- **Valid** — still holds as-is, no change needed
- **Needs update** — still relevant but wording/scope needs adjustment
- **Needs removal** — no longer applicable due to the change
- **New criterion needed** — the change introduces something not covered by existing criteria

Present the review in a clear format:

```
Re-evaluation: specs/graph-design/verification-criteria.md
트리거: _shared/graph-data-model — AddressMapRemap 엔티티 추가

- [P0-1] 기존 criterion 내용... → ✅ 유효
- [P0-3] 기존 criterion 내용... → ⚠️ 수정 필요 (remap 엔티티 반영)
- [P1-2] 기존 criterion 내용... → ❌ 삭제 (더 이상 해당 없음)
- [NEW]  제안 criterion 내용... → ➕ 신규 추가 필요
```

### 3. Level-specific Additional Checks

Based on the level of each affected spec, perform additional checks:

**When the affected spec is _shared:**
- Are all referencing features' verification-criteria covered?
- Are cross-feature consistency criteria still valid?
- Does a new entity/policy require corresponding criteria in referencing features?

**When the affected spec is a feature:**
- Do sub-feature verification-criteria need re-evaluation?
- Are cross-sub-feature criteria still valid?
- Does a boundary change with sibling features affect criteria?

**When the affected spec is a sub-feature:**
- Does the parent feature's verification-criteria need re-evaluation?
- Do sibling sub-feature verification-criteria need re-evaluation?
- Do criteria conflict with the parent's cross-sub-feature stories?

If these checks reveal **additional** affected specs not in the original list, report them to the user and include them in the review.

### 4. Escalation Assessment

For each affected spec, assess whether verification-criteria changes alone are sufficient:

**v-criteria update only** (handle here):
- Criterion wording adjustments
- New criteria for new elements
- Removal of obsolete criteria

**Spec modification needed** (escalate):
- Stories in spec.md are broken by the change
- Policies in spec.md conflict with the change
- References section needs updating
- New sub-features or structural changes needed

If escalation is needed, report to the user:
```
에스컬레이션 필요:
- specs/graph-design/spec.md — address-map 관련 story가 remap 엔티티를 반영하지 않음
  → feature-modify-interview 실행 권장
```

### 5. Present Results and Confirm

Present the complete re-evaluation results to the user:
- Per-spec summary of changes (valid/update/remove/new)
- Escalation items if any
- Proposed updated verification-criteria for each affected spec

Get user confirmation before writing any files.

### 6. Apply Changes

After user confirmation:
- Update each affected spec's verification-criteria.md
- Show diff for each file

### 7. Record in Work Plan

If a work plan exists for the current work:
- Record the re-evaluation results (which specs were reviewed, what changed)
- If escalation is needed, add modify phases for the affected specs (불가피한 경우의 후속 단계 수정)
- Mark this re-evaluation as completed

### 8. Reflection

Follow `../../reflection-protocol.md`.

Focus on:
- Did I miss an affected spec that the user had to add? (external signal)
- Did I miscategorize a criterion (e.g., marked valid but user corrected to needs-update)? (external signal)
- Did the level-specific checks reveal impacts I didn't anticipate? (self-judgment)
- Was escalation needed that I didn't detect? (either source)

## Interview Style

- **Start from the change.** Always anchor the review in what specifically changed.
- **Be systematic.** Review every criterion, don't skip ones that seem obviously fine.
- **Be conservative on escalation.** Only escalate when spec.md changes are genuinely needed, not for minor adjustments handleable through criteria updates.
- Present results **per-spec, per-criterion** — visual tables or checklists, not paragraphs.
- When uncertain whether a criterion is affected: ask the user rather than assuming it's fine.

## Output

- Updated `verification-criteria.md` for each affected spec
- Escalation report (if applicable) with recommended modify phases
- Work plan recording of results
- Updated `gotchas-re-eval.md` (via reflection, user confirmed)
