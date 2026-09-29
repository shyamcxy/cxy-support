# OneStack Agent Handoff

You are taking over an AI-first software runtime project.

## First 60 seconds

Read, in order:

1. `AGENTS.md`
2. `docs/agent-context.json`
3. `docs/product-spec.md`
4. `docs/technical-spec.md`
5. `docs/architecture.md`
6. `docs/task-protocol.md`
7. `docs/task-queue.json`

Then inspect the relevant GitHub Issue before coding.

## Your role

You are not writing a conventional web application.

You are helping build the runtime on which AI agents will build applications.

Think in:

`intent -> semantic graph -> execution semantics`

Do not immediately translate requirements into:

`route -> controller -> ORM -> queue -> worker -> framework code`

First ask:

- What semantic primitive represents this?
- What should the agent need to say?
- What should the runtime own?
- What information can be queried incrementally?
- How can another agent modify this later with minimal context?

## Current north star

An agent receives:

"Build a realtime support app with users, tickets, an AI triage worker."

It should be able to:

1. inspect compact project context;
2. pick a task;
3. mutate the application graph;
4. validate;
5. execute behavior;
6. verify;
7. leave a machine-readable handoff.

## Current branch

`onestack/v0.1-agent-core`

## Do not assume

- Postgres is implemented.
- GitHub task picking is implemented.
- CI has passed unless you can see the result.
- Production cloud infrastructure exists.
- Agent LLM execution is embedded in the runtime.

## Current remaining handoff tasks

- Issue #2: durable Postgres backend.
- Issue #8: GitHub-integrated machine task picker/handoff endpoint.

## Working rule

Take one issue, claim it, inspect existing implementation first, make the smallest coherent change, add tests, update contracts/docs, and leave a precise handoff.
