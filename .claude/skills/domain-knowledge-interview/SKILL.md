# Domain Knowledge Interview

Structure and organize domain knowledge so that intake and interview skills can reference it effectively during spec authoring.

## Trigger

When the user provides domain-specific documents (PDF, Excel, data sheets, internal specs) or describes domain knowledge that specs will need to reference.

## References

Always read:
- `./gotchas-domain.md` — Known domain structuring patterns

Read when needed:
- `../../spec-levels.md` § _domain — domain reference 파일 작성 시 (Step 4)

## Process

### 1. Read Existing Context

- If `specs/_domain/` exists, read all files to understand what domain knowledge is already captured
- This informs what is new vs what needs updating

### 2. Receive and Extract

Read all provided materials (documents, verbal descriptions, reference data). Extract:
- Key concepts and terminology
- Relationships and dependencies between concepts
- Constraints and rules (compatibility matrices, valid combinations, limits)
- Data tables and reference values

### 3. Present Structuring Plan

Before structuring, present the plan to the user:

```
도메인 지식 구조화 계획:
- {domain area 1}: {what will be captured, source}
- {domain area 2}: {what will be captured, source}
- ...

기존 domain reference와의 관계:
- {new / update to existing file / no change}

발견된 갭:
- {gap 1}
- {gap 2}

이 계획으로 진행할까요?
```

User confirms, modifies, or provides additional input.

### 4. Structure and Ask

Structure the domain knowledge into reference files. Each file covers one coherent domain area. File format follows `../../spec-levels.md` (_domain section).

While structuring, identify and ask about:
- Incomplete data: missing entries in tables, undefined relationships
- Related areas: concepts mentioned but not covered in provided materials
- Ambiguities: terms used inconsistently, unclear constraints
- **Domain fact vs service decision**: If the user states something that is a service-level design decision (e.g., limits, policies chosen by the team) rather than a domain fact, flag it and note it separately as a candidate for `_shared` scope. Do not record service decisions in `_domain`.

### 5. Save

Write domain reference files to `specs/_domain/`.
- New domain area → new file
- Existing domain area with new data → update file, refresh Last updated
- Outdated information replaced → update Status if needed

### 6. Reflection

Follow the protocol in `../../reflection-protocol.md`.

Focus on:
- Was the structuring useful to the user? (external signal)
- Were there domain concepts I misunderstood? (external signal)
- Did I miss obvious gaps? (self-judgment)
- Did the user correct my organization of concepts? (external signal)

## Output

1. Domain reference files at `specs/_domain/`
2. Gap list (confirmed by user)
3. Gotchas updates (via reflection, confirmed by user)

### 7. Re-evaluation Trigger

After updating domain reference files, check whether any `_shared` specs reference the changed `_domain` files (via their References section). If so, report to the user:

```
업데이트된 domain reference에 영향받는 _shared spec:
- {spec path} — {변경된 도메인 사실과의 관계}

re-eval-interview 실행을 권장합니다.
```

## Boundaries

- **Do**: Extract, organize, identify gaps, ask clarifying questions
- **Do not**: Derive planning direction from domain knowledge — that is the user's judgment
- **Do not**: Validate domain accuracy beyond what the provided documents state — that is the domain expert's responsibility
- **Do not**: Record service-level design decisions (limits, policies, modeling choices) in `_domain` — those belong in `_shared`. If they come up during the interview, note them separately and suggest they be addressed via spec-intake → shared-create/modify

## Gotchas

Read `./gotchas-domain.md` at the start of every execution.
