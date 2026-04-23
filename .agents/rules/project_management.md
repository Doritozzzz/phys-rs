---
trigger: always_on
glob:
description: Rule for project tracking, backlog synchronization, and preventing redundant work. Ensures the engineering state is always documented.
---

# Rule: Project Tracking & Documentation Integrity

This rule ensures that the project's progress is always visible and that the AI agent never repeats completed work or ignores established plans.

## 1. The Backlog as Truth

- **MANDATORY**: Before starting ANY task, the agent MUST read `docs/BACKLOG.md` to identify the current objective and verify if the task (or a related one) is already planned or completed.
- **MANDATORY**: Every code change MUST be reflected in `docs/BACKLOG.md`.
  - Mark completed tasks with `[x]`.
  - Add new requirements or discovered technical debt as new items in the appropriate phase.
  - If a task is modified during implementation, update its description in the backlog.

## 2. Preventing Redundancy

- **Verification Step**: Before creating a new file, module, or component, the agent MUST search the codebase for existing implementations of similar functionality.
- **Consultation**: If the user's request overlaps with an existing `[x]` item in the backlog, the agent must point this out and ask for clarification before proceeding.

## 3. Atomic Updates

- Every time a significant milestone is reached (e.g., a new system implemented, a bug fixed, a refactor completed), the agent MUST update `docs/BACKLOG.md` immediately. Do not wait for the end of the session.

## 4. Work Log Synchronization

- If the project uses a `ROADMAP.md` or `README.md` for high-level tracking, ensure these are updated in sync with the `BACKLOG.md` when a phase is completed.
