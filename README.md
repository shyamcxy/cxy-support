# OneStack

OneStack is an application runtime designed for AI agents as the primary software developers.

The core thesis:

> An agent should describe application intent once; the runtime should own the implementation details of data, APIs, jobs, events, realtime, auth, and UI.

## v0.1 goal

Prove:

`agent intent -> compact IR -> application graph -> executable runtime`

The first implementation deliberately avoids cloud infrastructure. It runs locally with an in-memory graph and exposes a machine-oriented API.

## Design principles

- Optimize for agent context, not human source-code ergonomics.
- Prefer stable semantic nodes over file-oriented reasoning.
- Make dependencies queryable as a graph.
- Keep the model-facing representation compact and deterministic.
- Treat data, actions, events, jobs, realtime, views, and agents as first-class primitives.
- Infrastructure is an implementation detail.

## Repository

- `crates/onestack-core` — application graph and IR
- `examples/` — tiny agent-authored applications

## First milestone

An AI agent can create, inspect, mutate, validate, and execute an application without manually managing a conventional web-stack directory tree.


## Current prototype

The v0.1 prototype has two layers:

```
AI agent
   ↓
NDJSON agent protocol
   ↓
Application Graph (control plane)
   ↓
Runtime (execution plane)
   ├── records
   ├── events
   └── workflow jobs
```

The same protocol can now mutate the graph and execute basic data/event operations.

Example:

```bash
printf '%s\n' \
  '{"id":1,"op":"put","node":{"kind":"Entity","id":"User","fields":{"email":{"ty":"String","required":true}}}}' \
  '{"id":2,"op":"create","entity":"User","data":{"email":"agent@example.com"}}' \
  '{"id":3,"op":"query","entity":"User"}' \
  | cargo run -p onestack-core -- agent
```

The exact wire schema is documented in [docs/agent-protocol.md](docs/agent-protocol.md).

### Next execution milestones

1. Action semantics: an Action node should compile directly into runtime operations.
2. Durable DB backend: swap the JSON store for Postgres without changing the agent protocol.
3. HTTP + WebSocket transport: expose the same runtime through one service.
4. Worker execution: execute workflow steps and agent tasks.
5. Generated UI: materialize Views into a frontend without exposing frontend framework details.
