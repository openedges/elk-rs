# Reflection Protocol

This document defines the shared reflection process used by all skills. Every skill references this protocol at the end of its execution.

---

## When to Trigger

Reflection runs once, at the end of every skill execution. Not during — only after the skill's primary task is complete.

---

## Process

### Step 1: Collect Observations

Gather from two sources equally:

**External signals** (higher reliability):
- User corrections during the session ("no, that should be X not Y")
- Repeated clarification questions (user had to explain the same thing multiple times)
- User explicitly flagging an issue ("this was wrong", "you missed X")

**AI self-judgment** (lower reliability, still valuable):
- Points where the AI was uncertain or made assumptions
- Patterns noticed across multiple interactions
- Mismatches between initial classification and final result

### Step 2: Compare with Existing Gotchas

Read the relevant gotchas file for this skill. For each observation:
- Is it already recorded? → Note as re-occurrence (count +1)
- Is it new? → Candidate for new entry
- Does it contradict an existing gotcha? → Flag for discussion

### Step 3: Propose Changes

Format each proposal as:

```
[New] {pattern description} (source: {external/self-judgment})
[Reoccurrence] {existing pattern} → current: [count/level], proposed: [count+1/new level]
[Contradiction] {existing pattern} conflicts with {new observation}
```

### Step 4: Present to User for Confirmation

Show all proposals to the user. Ask:
- Which proposals to accept?
- Any modifications?
- Any to reject?

Do NOT update gotchas without user confirmation.

### Step 5: Update Gotchas

For confirmed proposals, update the gotchas file using the semi-structured format:

```markdown
## 규칙 (3회+)
- {pattern} (source: {skill}, {count}회)

## 주의 (2회)
- {pattern} (source: {skill}, {count}회)

## 관찰 (1회)
- {pattern} (source: {skill}, {count}회)
```

### Step 6: Promotion Check

When an item's count crosses a threshold:
- 1→2: Propose 관찰→주의 promotion
- 2→3+: Propose 주의→규칙 promotion
- 규칙 promotion also means adding to the project's CLAUDE.md

Include promotion proposals in Step 4 (user confirms together with new entries).

---

## Rules

1. **Never update gotchas without user confirmation.**
2. External signals take priority over self-judgment when they conflict.
3. Keep entries concise — one line per pattern. Detail goes in the description, not the title.
4. Tag the source skill so patterns can be traced back.
5. If no observations worth recording → skip reflection. Don't force entries.
