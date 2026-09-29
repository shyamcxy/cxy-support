# OneStack Task Board

GitHub Issues are authoritative. `docs/task-queue.json` is the compact mirror.

## READY
- Issue #2 — Durable Postgres execution backend
- Issue #8 — Agent task picker and handoff endpoint
- Issue #9 — Object storage backend for File resources
- Issue #10 — First-class authorization policies

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
- Project task queue operations
- Long-running Job primitive
- File resource metadata primitive
- Realtime Job progress updates
- AI video application stress-test example

## Important
Items in IN PROGRESS / NEEDS VERIFICATION have implementation on their relevant branch but require a real Cargo/CI run before being marked DONE.
