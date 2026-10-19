# Reverse-spec Review

Review extracted specs via hybrid Propose-Verify and interview interaction. Transitions specs from `Extracted` to `Confirmed` after verification. Handles all spec levels (_shared, feature, sub-feature) in a single skill.

## Why a Single Skill

The review process is level-independent — driven by tags (`[추론]`, `[모순]`, empty sections) and plan phases, not by spec level. The same Propose-Verify flow applies whether reviewing a `_shared` entity definition or a sub-feature's stories. Gotchas are shared across levels. This follows the `re-eval-interview` precedent of handling all levels in one skill.

## Trigger

After `reverse-spec` generates draft specs and a reverse-spec plan. The user invokes this skill for specific plan items (one or more items per invocation), following the plan's phase order.

**Prerequisite**: A reverse-spec plan must exist at `specs/_work-plans/`. If the user invokes without a plan, ask what reverse-spec output they are reviewing.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `../../interview-protocol.md` — Research system, critical probes, interview style
- `../../interview-reference.md` § Propose-Verify — 기본 리뷰 모드
- `./gotchas-reverse-spec-review.md` — Known review patterns

Read when needed:
- `../../spec-levels.md` § {리뷰 대상 level} — 해당 level spec 리뷰 시
- `../../interview-reference.md` § Feature Map Update — 완료 시
- `../../interview-reference.md` § Spec State Management — 상태 전환 시
- `../../interview-reference.md` § Deferred Decision — deferral 발생 시
- `../../interview-reference.md` § Synthesis Checkpoints — 3라운드 이상 진행 시

**Critical context — read before each review session:**
- The reverse-spec plan — understand which items are assigned, phase context, any memos from prior reviews
- The target spec.md — current extracted content with tags
- Related specs (referenced specs, parent/child specs) — for cross-check context
- The codebase (relevant sections) — to provide code context during review

## Process

### 1. Read Plan and Identify Scope

Read the reverse-spec plan. Identify which items the user wants to review in this session. Confirm scope with the user.

Check for plan memos from prior reviews — if a previous review noted changes that affect this spec, incorporate that context.

### 2. Review Each Item

For each plan item in this session:

**Mode selection per section** (see Propose-Verify Mode in `interview-reference.md`):

| Section state | Mode | Approach |
|---------------|------|----------|
| Has content (no tag) | Propose-Verify | Present content with source context, ask for verification |
| Has content with `[추론]` | Propose-Verify | Present with explicit "이 내용은 코드 패턴에서 추론했습니다", ask for verification |
| Empty section | Interview | Provide code context, ask questions to elicit content |
| Has `[모순]` tag | Propose-Verify | Present both contradicting definitions side by side, ask for resolution |

**Per-section flow:**
1. Present the section content (or emptiness) with relevant code context
2. Apply critical probes (at least 1 per round, per `interview-protocol.md`)
3. Get user confirmation or correction
4. If confirmed → remove tag (content validated)
5. If corrected → update spec content, remove tag
6. If user cannot answer → apply Deferred Decision process (`interview-reference.md`)

### 3. Update Plan

After each item is reviewed, update the reverse-spec plan:

| Event | Update |
|-------|--------|
| Item completed | Status: `[ ]` → `[x]` |
| Item deferred | Status: `[ ]` → `[deferred]`, mark spec.md with `[DEFERRED]` |
| New gap discovered | Add item to appropriate phase with source annotation |
| Change affects other specs | Add plan memo: "{spec} Phase N에서 {변경 내용} → {대상 spec} 리뷰 시 반영 필요" |

Plan format for new items:
```markdown
#### {Phase}-NEW. `specs/{path}/spec.md` — 추가 항목
- **스킬**: reverse-spec-review
- **항목**: {설명}
- **출처**: Phase {N} 리뷰 중 발견
- **상태**: [ ] 미완료
```

### 4. Phase Completion Check

When all items in a phase are processed:
- Summarize: confirmed, deferred, newly discovered items
- **Phase 1 or 2**: Note any cross-spec change propagation for later phases (plan memo, NOT re-eval — all specs are still `Extracted`, no v-criteria exist)
- **Phase 3**: Check if re-eval is needed for any already-`Confirmed` specs (see Step 6)

### 5. Completeness Check (after Phase 3)

Run after Phase 3 items are completed for a spec:
- Are all `[추론]` tags resolved (confirmed or corrected)?
- Are all `[모순]` tags resolved?
- Are all empty sections filled or explicitly deferred?
- Are there deferred items that block v-criteria generation?

**If passed:**
- Generate `verification-criteria.md` for the spec (P0/P1/P2 per `spec-structure.md`)
- Transition Status: `Extracted` → `Confirmed`
- Record in plan: Completeness Check passed, v-criteria generated

**If not passed:**
- Report remaining gaps to user
- Suggest additional review rounds or deferred handling
- Do NOT generate v-criteria or transition status
- Record in plan: Completeness Check not passed, remaining items listed

### 6. Re-eval Assessment (Phase 3 only)

In Phase 3, specs are `Confirmed` sequentially. If a change during review of spec-B affects already-`Confirmed` spec-A:
- Report to user: "spec-A의 v-criteria가 재평가 필요할 수 있습니다 (spec-B의 {변경 내용}이 영향)"
- Suggest `re-eval-interview` for spec-A

Phase 1–2: No re-eval possible — all specs are `Extracted` with no v-criteria. Change propagation is handled via plan memos (Step 3).

### 7. Reflection

Follow the protocol in `../../reflection-protocol.md`.

Focus on:
- Did Propose-Verify mode cause anchoring bias? Did the user accept content uncritically that should have been challenged? (self-judgment)
- Did I miss contradictions between specs? (either source)
- Was the phase ordering effective? Did skipping phase order cause issues? (self-judgment)
- Did the user correct content I presented as high-confidence (no tag)? (external signal)
- Were critical probes effective in Propose-Verify mode vs standard interview mode? (self-judgment)

## Interaction Style

This skill uses a hybrid interaction model — not purely interview, not purely verification.

- **Default to Propose-Verify mode** for sections with content. Switch to interview mode only for empty sections.
- **Always state content source**: "코드 분석에서 직접 추출한 내용입니다" (no tag) vs "코드 패턴에서 추론한 내용입니다 ([추론])" (inferred).
- **Counter anchoring**: Ask "이 내용에서 틀리거나 빠진 부분이 있습니까?" — not "맞습니까?"
- **Be systematic**: Review every tagged item. Do not skip items that seem obvious.
- **Provide code context**: When presenting extracted content, show the relevant code that led to the extraction so the user can judge the mapping.
- **Apply critical probes**: At least 1 per round, same rigor as standard interviews. Category rotation per `interview-protocol.md`.

## Boundaries

- **Do**: Review extracted content with user, verify and correct tags, generate v-criteria after Completeness Check, update reverse-spec plan (mark complete, add gaps, propagation memos), trigger re-eval for `Confirmed` specs affected by Phase 3 changes
- **Do not**: Modify spec structure or feature-map — that was reverse-spec's job, confirmed by user
- **Do not**: Generate v-criteria before Completeness Check passes
- **Do not**: Run re-eval during Phase 1–2 — no `Confirmed` specs exist, no v-criteria to re-evaluate
- **Do not**: Skip tagged items even if they appear correct — every tag must be explicitly resolved with the user

## Output

1. Updated `spec.md` files (tags resolved, content confirmed/corrected, Status: `Extracted` → `Confirmed` after Completeness Check)
2. `verification-criteria.md` for specs that pass Completeness Check (P0/P1/P2)
3. Updated reverse-spec plan (completion status, memos, new items, Completeness Check results)
4. Updated `gotchas-reverse-spec-review.md` (via reflection, user confirmed)

## Gotchas

Read `./gotchas-reverse-spec-review.md` at the start of every execution.
