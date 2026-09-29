# OneStack Task Protocol

GitHub Issues are executable work units for AI agents.

## Task states
Use the issue state plus explicit section markers:
- READY — safe for an agent to start.
- IN PROGRESS — an agent has claimed it.
- BLOCKED — cannot proceed because a dependency or decision is unresolved.
- DONE — acceptance criteria verified.

## How an agent claims work
Before coding:
1. read `AGENTS.md`;
2. inspect the issue;
3. check dependencies;
4. comment on the issue with `CLAIM: <agent-id>`;
5. change the issue body state to IN PROGRESS if appropriate.

Do not claim multiple tasks when their work overlaps unless deliberately coordinating.

## Issue contract
Every executable issue should contain:
- Goal
- Why
- Context
- Scope
- Non-goals
- Files/areas
- Dependencies
- Acceptance criteria
- Verification commands
- Handoff notes

## Completion protocol
When done:
1. run verification commands;
2. update relevant docs;
3. comment with `DONE: <summary>`;
4. include tests and commit/PR references;
5. identify remaining risks.

## Agent autonomy
An agent may choose implementation details within the issue scope.

It must stop and mark BLOCKED when:
- the product contract is ambiguous;
- required credentials/infrastructure are unavailable;
- another issue owns the necessary interface;
- making the change would violate an architectural boundary.

## Board rule
Issues are the durable task queue.

Humans discuss product direction here.

Agents execute READY issues.

The repository plus issue history should be enough for a fresh agent to continue the project.

## Current machine-visible task queue

GitHub Issues remain the authoritative task records. The repo keeps a compact mirror at `docs/task-queue.json` for agents that need cheap context before calling external project-management tools.

An agent should prefer one task at a time. When an issue is IN PROGRESS, inspect its comments before duplicating work.

## Completion status semantics

- READY: untouched and safe to claim.
- IN PROGRESS: implementation may already exist; verify, finish, or continue it rather than restarting blindly.
- DONE: acceptance criteria and verification are complete.
- BLOCKED: explicit external dependency or unresolved contract.
