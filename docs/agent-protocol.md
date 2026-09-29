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
