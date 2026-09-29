# OneStack Architecture

## Overview

```text
                  AI Agent
                     |
              Agent Protocol
                     |
             Application Graph
                     |
          +----------+----------+
          |                     |
     Control Plane         Execution Plane
          |                     |
      inspect/patch         records/events
      validate              jobs/actions
          |                     |
          +----------+----------+
                     |
               infrastructure
```

## Repository boundaries
`crates/onestack-core/src/lib.rs`
- graph primitives and core types.

`engine.rs`
- OIR parser and validation.

`protocol.rs`
- agent control protocol.

`runtime.rs`
- execution semantics.

`bin/onestack.rs`
- local process and persistence.

## Important boundary
The agent protocol MUST NOT directly expose SQL, Redis, Kafka, Kubernetes, or cloud-provider-specific resources unless an explicit escape-hatch design is accepted.

The protocol represents intent.

## Planned execution stack
- Rust runtime;
- Postgres;
- HTTP;
- WebSocket;
- durable workflow queue;
- object storage;
- generated TypeScript frontend.

The exact underlying providers remain implementation details.

## Future compiler
A later compiler may map graph nodes to tables/indexes, routes, event handlers, workers, WebSocket subscriptions, auth checks, generated UI, and deployment units.

The graph should remain the canonical source for semantic behavior.

## Agent context architecture
Agents should not load the repository blindly.

Preferred sequence:
1. read `AGENTS.md`;
2. read compact `agent-context.json`;
3. inspect only relevant graph nodes;
4. open implementation files only when needed;
5. complete one issue;
6. update task state and docs.

This is itself a product requirement.
