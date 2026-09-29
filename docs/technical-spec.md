# OneStack Technical Specification

## System model
OneStack has two planes.

### Control plane
The machine-facing application graph and protocol.

### Execution plane
The runtime that executes graph semantics.

The two planes must remain decoupled.

## Graph primitives
### Entity
Persistent structured data with fields and relationships.

### Action
A semantic operation with typed input, writes, emitted events and optional auth requirements.

### Event
A semantic fact that can trigger workflows and realtime propagation.

### Workflow
A relation from an event to ordered execution steps.

### Agent
A first-class execution actor with declared reads and writes.

### View
A data-backed read surface, optionally realtime.

## Current wire protocol
NDJSON request/response.

Example:
```json
{"id":1,"op":"inspect","node_id":"Ticket"}
```

Response:
```json
{"id":1,"ok":true,"result":{}}
```

The protocol is semantic and transport-oriented rather than tied to HTTP semantics.

## Required properties
- deterministic request/response;
- stable node identifiers;
- structured errors;
- incremental inspection;
- explicit graph dependencies;
- idempotent upsert semantics where practical;
- validation before execution;
- testable execution semantics.

## Runtime evolution
Current: `App + Runtime + local JSON persistence`
Target: `App Graph + OneStack Runtime + Postgres + workers + HTTP/WebSocket + generated UI`

The agent protocol should remain stable while the execution backend evolves.

## Persistence
v0.1 persists local state as JSON.

The Postgres milestone must preserve semantic operations such as:
- create entity record;
- query entity;
- invoke action;
- emit event;
- enqueue workflow job.

The agent must not need to know SQL, migrations, connection pools, or queue providers.

## Error model
Errors must be structured and actionable.

Bad: `something failed`

Good: `action createTicket references missing entity Ticket`

The runtime should eventually expose machine-readable error codes, affected nodes, and suggested remediation.

## Performance model
The primary optimization target is agent interaction cost.

A protocol operation should return only the information needed for the next reasoning step.

Full application snapshots are an explicit escape hatch, not the normal workflow.
