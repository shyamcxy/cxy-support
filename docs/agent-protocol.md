# OneStack Agent Protocol v0.1

OneStack is intentionally controlled through a small structured protocol instead of a human-oriented framework API.

## Transport

The first transport is NDJSON over stdin/stdout.

One request = one JSON object on one line.
One response = one JSON object on one line.

Example:

```json
{"id":1,"op":"list"}
{"id":2,"op":"inspect","node_id":"Ticket"}
{"id":3,"op":"validate"}
```

## Operations

### list

Returns node ids.

```json
{"id":1,"op":"list"}
```

### inspect

Returns a node plus direct dependencies and dependents.

```json
{"id":2,"op":"inspect","node_id":"Ticket"}
```

### deps

Returns direct dependencies and dependents.

```json
{"id":3,"op":"deps","node_id":"Ticket"}
```

### put

Upserts a complete node.

```json
{
  "id":4,
  "op":"put",
  "node":{
    "kind":"Event",
    "id":"ticket.created"
  }
}
```

### patch

Applies a graph patch.

```json
{
  "id":5,
  "op":"patch",
  "patch":{
    "op":{
      "Upsert":{
        "kind":"Event",
        "id":"invoice.created"
      }
    }
  }
}
```

### validate

Checks graph references and primitive-specific constraints.

```json
{"id":6,"op":"validate"}
```

### create

Creates a record in an Entity using the runtime's validated data model.

```json
{"id":7,"op":"create","entity":"Ticket","data":{"message":"hello"}}
```

### query

Reads records for an Entity.

```json
{"id":8,"op":"query","entity":"Ticket"}
```

### emit

Emits an application event and schedules matching workflow steps.

```json
{"id":9,"op":"emit","event":"ticket.created","payload":{"ticket":"r1"}}
```

### next_job

Takes the next workflow job from the execution queue.

```json
{"id":10,"op":"next_job"}
```

### snapshot

Returns the whole application graph.

Use this sparingly. The protocol is designed so an agent can inspect only the semantic nodes it needs instead of loading the entire project.

## Design constraint

The protocol is not intended to mirror Rust, TypeScript, SQL, Terraform, or a cloud provider API.

It is the control surface for an AI developer.

The runtime may later compile the same graph into:

- database schema
- API endpoints
- background workers
- event delivery
- WebSockets
- auth
- UI
- deployment infrastructure

Those are implementation details.

## Local usage

```bash
cargo run -p onestack-core -- import examples/support.oir
printf '%s\n' '{"id":1,"op":"list"}' '{"id":2,"op":"validate"}' | cargo run -p onestack-core -- agent
```

The application state is persisted to `.onestack/app.json`.

### execute_job

Executes one queued workflow job. Action steps run inside the semantic runtime. Agent steps become an AgentTask for an external AI executor.

```json
{"id":11,"op":"execute_job"}
```

### next_agent_task

Returns the next queued AgentTask.

```json
{"id":12,"op":"next_agent_task"}
```

### subscribe / unsubscribe / next_update

Manage realtime View subscriptions.

```json
{"id":13,"op":"subscribe","view":"tickets"}
{"id":14,"op":"next_update","subscription_id":1}
{"id":15,"op":"unsubscribe","subscription_id":1}
```

### list_tasks / next_task / seed_tasks / claim_task / complete_task

Machine task picker for GitHub Issues without calling external APIs.
`seed_tasks` loads the compact mirror (`docs/task-queue.json`).
`next_task` returns the lowest READY issue. `claim_task` moves READY → IN_PROGRESS.
`complete_task` moves IN_PROGRESS → DONE or BLOCKED with handoff notes.

```json
{"id":16,"op":"seed_tasks","payload":[{"issue":2,"title":"Durable Postgres execution backend","state":"READY"}]}
{"id":17,"op":"list_tasks"}
{"id":18,"op":"next_task"}
{"id":19,"op":"claim_task","issue":2,"agent":"agent-1"}
{"id":20,"op":"complete_task","issue":2,"task_state":"DONE","handoff":"postgres trait landed"}
```

### HTTP transport

The same protocol is exposed at:
- `GET /health`
- `POST /agent`

Run locally with:

```bash
cargo run -p onestack-core --bin onestack -- http 127.0.0.1:8787
```
