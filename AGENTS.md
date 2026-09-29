# OneStack — Agent Operating Context

You are an AI coding agent working on OneStack.

## Mission
Build an application runtime whose primary developer is an AI agent.

The agent should express application intent through a compact semantic model. OneStack should handle implementation details such as data persistence, APIs, events, workers, realtime, auth, UI, and deployment.

## North-star test
Given a natural-language application request, an agent should be able to:
1. understand the existing application from structured context without reading the entire repository;
2. inspect and mutate a semantic application graph;
3. execute and verify behavior;
4. make a focused change with minimal context and tool calls;
5. leave the repository in a state another agent can continue from.

Optimize for fewer tokens, fewer tool calls, less repeated context, determinism, explicit dependencies, machine-readable state, and safe incremental changes.

## Source of truth
1. `docs/agent-context.json` — compact machine context.
2. `docs/product-spec.md` — product definition.
3. `docs/technical-spec.md` — technical contract.
4. `docs/architecture.md` — architecture and boundaries.
5. `docs/task-protocol.md` — how tasks are claimed and completed.
6. GitHub Issues — executable work items.

## Current implementation
- application graph;
- Entity / Action / Event / Workflow / Agent / View primitives;
- dependency and dependent queries;
- OIR parser;
- graph validation;
- NDJSON agent protocol;
- local persisted state;
- record creation/query;
- event emission;
- workflow job queue;
- semantic Action invocation.

## Guardrails
- Do not silently redesign the product.
- Do not introduce infrastructure dependencies before a task requires them.
- Preserve the agent protocol as the stable control surface.
- Prefer one semantic primitive over multiple framework-specific abstractions.
- Every new behavior needs tests.
- Every task must update relevant docs when the contract changes.
- Do not claim a build/test passed unless it was actually run.

## Definition of done
Implementation exists, tests cover the behavior, relevant docs/contracts are updated, acceptance criteria are satisfied, and no known regression is introduced.
