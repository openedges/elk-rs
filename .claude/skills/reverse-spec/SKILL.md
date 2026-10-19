# Reverse-spec

Extract draft spec.md files from existing codebases to provide a starting point for spec-workflow onboarding.

## Trigger

When the user has an existing service or codebase without spec-workflow compatible specs and wants to onboard it into the spec-workflow system. Not tied to intake or a work plan — invoked independently as the entry point for existing service onboarding.

**Do not use when:**
- The project has no code yet (new project) — use spec-intake instead
- Spec-workflow compatible specs already exist for the codebase — use spec-intake for changes

## References

Always read:
- `../../spec-structure.md` — Core structure definitions
- `./gotchas-reverse-spec.md` — Known reverse-spec patterns

Read when needed:
- `../../spec-levels.md` — 전체 (Step 3 Extract Draft Specs 진입 시, 모든 level의 spec.md 템플릿 필요)

Also read for context:
- The target codebase (user specifies path/scope)
- `specs/_domain/` — domain reference files (if any exist)

## Process

### 1. Analyze Codebase

Scan the target codebase to understand its structure:
- Identify logical modules, shared data structures, feature boundaries
- Map code dependencies (imports, shared modules, DB schemas)
- Determine decomposition order based on code analysis:
  - If shared data models (DB schemas, common modules) are clearly defined and referenced by multiple modules → extract `_shared` first
  - If features are self-contained with minimal sharing → extract features first
  - If mixed → propose both in parallel at outline level, then detail after user confirmation

Present the proposed decomposition order to the user with rationale. Get confirmation before proceeding.

### 2. Propose Feature Decomposition

Map code modules to spec-workflow levels:
- **_shared**: shared data models, common business logic referenced by 2+ features
- **feature**: cohesive units of functionality with independent purpose
- **sub-feature**: specific capabilities within a feature

Present the proposed structure as a feature-map draft. Include for each entry:
- Proposed name
- Code modules that map to it
- Rationale for the level assignment

Get user confirmation. Adjust based on feedback. Create initial `feature-map.md` based on confirmed structure.

### 3. Extract Draft Specs

For each spec in the confirmed order, create `spec.md` following `spec-levels.md` format for the appropriate level.

**Extraction confidence tags:**

| Tag | Meaning | When to use |
|-----|---------|-------------|
| (no tag) | Content directly observable in code with high confidence | Validation rules, explicit conditionals, clear API contracts |
| `[추론]` | Content inferred from code patterns, naming, or indirect evidence | Purpose derived from function names, policies inferred from error handling patterns |
| (empty section) | Content not extractable from code | Leave blank — do NOT fill with hallucinated content. Use `<!-- 코드에서 추출 불가 -->` placeholder |

Set Status to `Extracted` for every generated spec.

Do NOT generate `verification-criteria.md` — that is `reverse-spec-review`'s responsibility after Completeness Check.

### 4. Cross-check

After all specs are extracted:
- Compare specs against each other for contradictions (e.g., spec-A's policy conflicts with spec-B's policy)
- Tag contradictions with `[모순]` inline at the relevant section, with a cross-reference to the contradicting spec:
  ```
  [모순] 이 정책은 `_shared/{name}` spec.md의 {section}과 충돌합니다: {설명}
  ```
- Do NOT tag `[개선 필요]` — improvement judgments are the planner's domain
- Tally results: count of `[추론]` tags, empty sections, and `[모순]` tags per spec

### 5. Generate Reverse-spec Plan

Create a plan document at `specs/_work-plans/{date}-reverse-spec-{service-name}.md`.

**Plan format:**

```markdown
# Reverse-spec Plan: {service name}

**생성일**: {date}
**대상**: {코드베이스 경로/범위}

## 추출 요약
- **추출된 spec**: {count}개 (feature N, sub-feature N, _shared N)
- **[추론] 항목**: {count}개
- **빈 섹션**: {count}개
- **[모순]**: {count}개

## 검토 가이드

### Phase 1 — 기반 확립
Purpose 확정 및 빈 섹션 보충. 이후 판단의 기준이 되므로 최우선.

#### 1-1. `specs/{path}/spec.md`
- **스킬**: reverse-spec-review
- **항목**: {이슈 유형과 요약}
- **상태**: [ ] 미완료

### Phase 2 — 추론 검증
AI가 추론한 내용의 정확성 확인.

#### 2-1. `specs/{path}/spec.md`
- **스킬**: reverse-spec-review
- **항목**: {[추론] 항목 요약}
- **상태**: [ ] 미완료

### Phase 3 — 모순 해소
추출된 정의 간 충돌 해소. Phase 1~2 완료 후 진행.
Phase 3 내 순서: _shared → feature → sub-feature (상위 확정 후 하위 진행).

#### 3-1. `specs/{path}/spec.md`
- **스킬**: reverse-spec-review
- **항목**: {[모순] 요약 + 충돌 대상}
- **상태**: [ ] 미완료
```

**Phase ordering principle** (Definition Priority):
- Phase 1 first: Purpose and foundations must be established before other judgments
- Phase 2 next: Verify inferred content against established foundations
- Phase 3 last: Contradictions are consequences of preceding definitions — resolve after definitions stabilize

### 6. Present to User

Present complete results:
- Feature-map (confirmed structure)
- All extracted draft spec.md files (with tag counts)
- Reverse-spec plan with phase breakdown
- Summary statistics

Get user confirmation before writing files.

### 7. Reflection

Follow the protocol in `../../reflection-protocol.md`.

Focus on:
- Did the decomposition match the user's mental model? (external signal)
- Were there code modules I misclassified? (external signal)
- Was the _shared-first vs feature-first decision appropriate? (self-judgment)
- Did I hallucinate content where I should have left sections empty? (self-judgment)
- Did I miss cross-spec contradictions found later? (either source)

## Boundaries

- **Do**: Analyze code structure, extract observable behavior into spec format, tag uncertainty honestly, generate reverse-spec plan, create feature-map (with user confirmation)
- **Do not**: Fill empty sections with guessed content — leave them blank with placeholder
- **Do not**: Generate verification-criteria.md — that is reverse-spec-review's responsibility after Completeness Check
- **Do not**: Judge what "should be improved" — tag only `[모순]` (objective contradictions), not `[개선 필요]` (subjective improvement)
- **Do not**: Act on existing non-code documents (Figma, PPT, text docs) — input is code only
- **Do not**: Transition Status beyond `Extracted` — confirmation is reverse-spec-review's responsibility

## Output

1. Draft `spec.md` files for each identified spec (with `[추론]`/`[모순]` tags, Status: `Extracted`)
2. `feature-map.md` (initial, code-analysis-based, user confirmed)
3. Reverse-spec plan at `specs/_work-plans/{date}-reverse-spec-{service-name}.md`
4. Updated `gotchas-reverse-spec.md` (via reflection, user confirmed)

## Gotchas

Read `./gotchas-reverse-spec.md` at the start of every execution.
