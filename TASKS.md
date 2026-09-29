# OneStack Task Board

GitHub Issues are authoritative. `docs/task-queue.json` is the compact mirror.

## READY
- Issue #2 — Durable Postgres execution backend
- Issue #8 — Agent task picker and handoff endpoint

## IN PROGRESS / NEEDS VERIFICATION
- Issue #3 — HTTP transport
- Issue #4 — structured machine-readable errors
- Issue #5 — workflow job + Agent step execution
- Issue #6 — realtime subscription semantics
- Issue #7 — agent interaction benchmark

## DONE IN CODE
- Application graph primitives
- OIR parser
- Graph validation
- NDJSON agent protocol
- Local runtime records
- Event emission
- Workflow job queue
- Semantic action invocation
- HTTP transport
- Stable protocol error payloads
- Workflow job execution
- AgentTask queue
- Realtime subscription/update primitives
- Deterministic interaction-cost benchmark harness

## Important
Items in IN PROGRESS / NEEDS VERIFICATION have implementation on `onestack/v0.1-agent-core` but require a real Cargo/CI run before being marked DONE.
