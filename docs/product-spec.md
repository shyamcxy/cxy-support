# OneStack Product Specification

## Problem
AI coding agents currently operate through human-oriented software boundaries: repositories full of framework files, separate APIs, ORM models, queues, infrastructure definitions, and deployment configuration.

That forces an agent to spend context understanding implementation plumbing instead of application intent.

## Product
OneStack is a single application runtime and control plane designed around AI agents.

The agent describes what the application is and what it should do. OneStack owns the implementation of data, APIs, events, jobs, realtime, UI, and infrastructure.

## Primary user
The primary user is an AI software agent.

Humans are observers, reviewers, requirement providers, and operators.

## Core interaction
Human: "Build a realtime support application with users, tickets and an AI triage worker."

Agent:
- inspects application context;
- adds semantic nodes;
- validates the graph;
- executes behavior;
- runs tests;
- reports results.

The agent should not need to hand-author framework boilerplate.

## Product principles
### 1. Intent over implementation
The control surface describes application behavior rather than framework mechanics.

### 2. Semantic graph over file graph
The agent should be able to ask:
- What uses Ticket.status?
- What breaks if Ticket is changed?
- What actions create Ticket?
- Which workflows fire from ticket.created?

### 3. One control plane
Data, actions, events, jobs, realtime, UI and future infrastructure are controlled through one protocol.

### 4. Implementation is replaceable
A semantic application must survive replacement of local storage with Postgres, local execution with distributed workers, or one transport with another without changing the agent mental model.

### 5. Agent-first economics
Every operation should be designed around token/context/tool-call cost.

## v0.1 product boundary
In:
- application graph;
- OIR;
- agent protocol;
- graph validation;
- local runtime;
- records;
- events;
- workflow jobs;
- action invocation;
- task handoff system.

Out:
- production cloud;
- multi-region;
- billing;
- hosted dashboard;
- arbitrary code generation;
- advanced auth;
- production-grade distributed scheduling.

## Success criteria
A benchmark agent can build and modify a small application through OneStack while requiring materially less context and manual repository navigation than a conventional stack.

Track:
- tokens consumed;
- context loaded;
- tool calls;
- edits;
- failures;
- time to verified behavior.
