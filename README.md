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
