# Brainstorming

Help users explore and develop service-related ideas in a free-form conversation, while referencing existing specs and domain knowledge for context.

## Trigger

When the user wants to explore ideas, brainstorm, or develop an unvalidated concept. Not tied to intake or a work plan — can be invoked independently.

## References

Always read:
- `../../spec-structure.md` — Core structure definitions (기존 spec 이해용)
- `./gotchas-brainstorming.md` — Known brainstorming session patterns

Read when needed:
- `../../spec-levels.md` § _ideas — idea 저장 시 format 확인 (Step 5)

Also read for context:
- `specs/_ideas/` — existing ideas (if any)
- `specs/_domain/` — domain reference files (if any)
- `specs/` top-level folders — current feature landscape
- The project's `feature-map.md` (in spec-intake skill folder) — feature boundaries

## Process

### 1. Read Context

- Read `specs/_ideas/` to understand what ideas already exist
- Scan `specs/` folders and `feature-map.md` for the current project landscape
- Read `specs/_domain/` for domain context
- Read `gotchas-brainstorming.md`

### 2. Identify Session Scope

- If the user specified an existing idea (by name or file path) → load that idea file for continued development
- If the user's intent is ambiguous → ask: "새 아이디어를 탐색하려는 건가요, 기존 아이디어를 발전시키려는 건가요?"
- If new idea → prepare to create a new idea file
- A session may cover multiple ideas — follow the user's lead

### 3. Free-form Conversation

Engage in open-ended conversation about the user's idea. The user drives the direction.

**Your role:**
- Listen and help articulate the idea
- When relevant, surface connections to existing specs or domain knowledge (shallow feasibility check): "이 아이디어는 `{spec}`의 `{entity}`와 연결될 수 있을 것 같습니다"
- Note potential references as they emerge
- If the user switches to a different idea, follow naturally

**Shallow feasibility check** (observation level only):
- Note which existing entities an idea could reuse
- Flag if a field or concept doesn't exist in current specs
- Do NOT perform impact analysis or cascade assessment — that is intake's scope

Follow the Boundaries section below — do not interview, do not push toward intake.

### 4. Apply Soft Probes (When Appropriate)

Use Soft Probes as opportunities, not obligations. See the Soft Probes section below for categories and activation guidance.

### 5. Save

When the session reaches a natural conclusion or the user requests saving:

- Create or update idea files in `specs/_ideas/`
- Use the idea artifact format (see below)
- Set Status based on the idea's development level
- Present the file to the user for review before writing

**Idea artifact format:**

```markdown
# {Idea Title}

**Status**: Draft | Developed
**Created**: {date}
**Last updated**: {date}

## 배경
왜 이 아이디어가 떠올랐는가. 어떤 문제를 해결하려는가.

## 아이디어
자유 형식. 사용자가 원하는 만큼 상세하게 또는 간략하게.

## 관련 참조
- `_domain/{name}` — {관련성}
- `_shared/{name}` — {관련성}
- `feature/{name}` — {관련성}

## 메모
추가 생각, 열린 질문, 떠오르는 우려 등.
```

**Status criteria:**
- `Draft` — still being explored, may change significantly
- `Developed` — articulated well enough to be acted upon (intake, team discussion, etc.)

### 6. Reflection

Follow the protocol in `../../reflection-protocol.md`.

Focus on session quality:
- Did I miss obvious connections to existing specs? (self-judgment)
- Did the user have to point out relevant domain knowledge I should have surfaced? (external signal)
- Did I over-structure or push the conversation toward interview territory? (either source)
- Were Soft Probes used effectively, or too aggressively / not at all? (self-judgment)

## Soft Probes

5 categories of optional probes, drawn from research on creative constraints (Oxford IJQHC 2019, IConIC model). Each has a functional type: **Focusing** (directs attention toward something useful) or **Exclusionary** (surfaces something to watch out for).

| # | Category | Function | Example |
|---|----------|----------|---------|
| 1 | Context Link | Focusing | "이 아이디어는 기존 `{feature}`와 연결될 수 있을 것 같은데, 확인해볼까요?" |
| 2 | User Perspective | Focusing | "이걸 실제로 사용할 사람은 누구인가요?" |
| 3 | Alternative Exploration | Exclusionary | "다른 접근으로 {X}도 가능한데, 어떻게 보시나요?" |
| 4 | Uncertainty Recognition | Exclusionary | "이 아이디어에서 가장 불확실한 부분은 뭘까요?" |
| 5 | Feasibility Signal | Focusing | "기존 `{entity}`를 재사용할 수 있을 것 같습니다" |

### Activation Guidance

- **When**: the user seems stuck, a natural topic transition occurs, or the user asks for feedback
- **How**: weave probes into the conversation naturally — don't announce "now I'm applying a probe"
- **Restraint**: do not force probes. If the user is flowing freely, let them flow
- Specific activation criteria are intentionally undefined — observe patterns via reflection and accumulate gotchas as the skill matures

## Boundaries

- **Do**: listen, help articulate, surface relevant spec/domain connections, record ideas
- **Do**: shallow feasibility check — note entity reuse potential, flag missing concepts
- **Do not**: perform impact analysis or cascade assessment — that is intake's scope
- **Do not**: push toward intake or spec creation — the user decides when an idea is ready
- **Do not**: derive planning direction from ideas — that is the user's judgment
- **Do not**: turn the session into an interview — no mandatory questions, no forced structure

## Output

1. Idea files at `specs/_ideas/{idea-title}.md` (created or updated)
2. Updated gotchas (via reflection, user confirmed)

## Gotchas

Read `./gotchas-brainstorming.md` at the start of every execution.
