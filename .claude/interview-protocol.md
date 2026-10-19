# Interview Protocol

This document defines the core interview methodology used by all interview skills and reverse-spec-review. Each skill references this protocol for research, critical probing, and interview style — while owning its own process steps and scope.

Conditional/lookup sections (Synthesis Checkpoints, Feature Map Update, Spec State Management, Propose-Verify, Deferred Decision) are in `interview-reference.md`.

---

## Definition Priority

When defining a spec, there is a natural dependency between levels of definition. Higher levels must be established before lower levels can be meaningfully defined.

### Default Order

1. **Purpose** — Why this exists. All subsequent decisions are judged against this.
2. **Core expected behavior** — What happens when this works correctly. Includes key assumptions, input/output, and alternative paths. Flow dependencies ("where does this input come from?") surface here naturally.
3. **Edge cases, abuse, and failure** — What happens when things go wrong. Requires #1 and #2 to be defined first.

Structural dependencies (_shared references, cross-feature interactions) are identified by intake and confirmed during #2.

### The Principle

**Do not finalize lower levels before higher levels are established.** Specifically: do not commit to edge case handling before the core expected behavior is defined, and do not define behavior before the purpose is clear.

This is a dependency constraint, not a rigid sequence. If the user starts from an edge case ("users are abusing our refund policy"), that's fine — but recognize the edge case as a signal about purpose and work upward before working downward.

### Adaptation by Skill Type

- **Create interviews**: This order applies naturally. Start from purpose.
- **Modify interviews**: Purpose and behavior already exist. The entry point is the **change** — but verify that the change doesn't conflict with established purpose before proceeding to edge cases.
- **_shared interviews**: "Core expected behavior" translates to "entity relationships and constraints" rather than user flows.

---

## Research System

### When to Research

**Pre-interview** (before Round 1):
- Search for industry practices related to the feature/domain being defined
- Identify common patterns, pitfalls, and alternatives
- Share 1-2 key findings to set context

**Per-round** (reactive, based on user answers):
- When user states a policy → search for evidence supporting or contradicting it
- When user seems locked into one approach → search for alternatives
- When user describes a pattern → search for real-world case studies

**At critical decisions** (when a major policy is being set):
- Search for post-mortems from similar decisions
- Search for comparison articles between options considered

### Research Rules

- Use WebSearch for discovery, WebFetch for deep reading of specific articles
- **2-4 searches per round** depending on complexity. Don't over-research simple questions.
- **Always cite sources.** When sharing findings, include where the information came from.
- **Never fabricate.** If search yields nothing useful, say so honestly.
- Prefer recent sources (within 2 years), primary sources over aggregators.
- Distinguish between established best practices, emerging trends, and opinions.

### When Research Finds Nothing

- Be transparent: "I searched but didn't find strong evidence. Let's reason from first principles."
- Use adjacent evidence: "Related domain [X] has research showing [Y]. This might apply."
- Flag for validation: "No established best practice here — recommend validating with a prototype."

---

## Critical Probe System

Every interview round MUST include at least 1 critical probe out of 2-4 total questions. Critical probes are drawn from 6 categories.

### Categories

**1. Assumption Challenge**
Challenge unstated assumptions with evidence.
- "You're assuming [X] — what evidence supports this?"
- "[Research] shows this isn't always true because..."

**2. Alternative Path**
Force consideration of alternatives.
- "Other products handle this differently — [example]. Why your approach?"
- "Have you considered [alternative]? [Evidence] suggests..."

**3. Failure Scenario**
Explore what happens when things go wrong.
- "What if this policy fails? What's the user experience?"
- "[Company] had this exact failure — [case study]. How would you handle it?"

**4. Scale/Load Pressure**
Push on growth and edge volumes.
- "What happens with 10x the expected data?"
- "This policy works for 10 items. What about 10,000?"

**5. User Abuse**
Think like a user who misuses the system.
- "How could a user game this policy?"
- "What if someone intentionally triggers [edge case]?"

**6. Hidden Dependency**
Uncover non-obvious coupling.
- "This depends on [X] — what if [X] changes?"
- "Who else uses this data? What breaks if you change it?"

### Category Rotation

- Track which categories have been used across rounds.
- Prioritize uncovered categories.
- By the end of the interview, aim to cover at least 4 of 6 categories.

---

## Interview Style

### Core Principles

- **Ask 2-4 questions per round.** Don't overwhelm the user.
- **Include 1+ research-backed observation per round** when relevant research exists.
- **Challenge, don't interrogate.** Frame challenges as "have you considered..." not "you're wrong."
- **Propose when the user is stuck.** Offer 2-3 options with trade-offs rather than waiting.
- **Follow up on vague answers immediately.** Do not skip to the next topic.
- **Policies must include rationale.** When a policy is defined, always ask "why this choice?" and record the reasoning alongside the policy. A good policy with rationale implicitly documents which alternatives were considered and rejected (e.g., "3회 재시도 — 5회는 사용자 불만 유발, 1회는 오류 복구 불충분"). No separate "rejected alternatives" section needed.

### Handling Ambiguity

When the user is uncertain:
- Offer researched options: "Based on my research, here are 3 common approaches: [A], [B], [C]. Which resonates?"
- If the user genuinely doesn't know, provide a recommendation with rationale and log it as an open item.
- Never force a decision — mark it as open and move on if needed.
- If the user needs external input that cannot be resolved in this session, use the Deferred Decision process (see `interview-reference.md` § Deferred Decision Handling).

### Domain Reference Check

If `specs/_domain/` exists, compare user inputs against domain reference files during the interview. Flag domain concepts mentioned by the user but not documented in domain reference — regardless of whether the user is uncertain. Treat `[UNVERIFIED]` sections as reference only, not as established fact.

---

### Spec-Context Adaptations

This protocol is used for **spec interviews** (purpose, policies, stories), not technical architecture interviews. Adapt accordingly:
- Research focuses on **business policy patterns, UX practices, domain conventions** — not technology stack choices
- Critical probes focus on **policy completeness, user experience gaps, edge cases** — not performance benchmarks
- Synthesis focuses on **spec completeness and consistency** — not technical feasibility
