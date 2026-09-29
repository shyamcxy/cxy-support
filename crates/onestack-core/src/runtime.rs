use crate::{App, FieldType, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, VecDeque};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", content = "details")]
pub enum RuntimeError {
    #[error("entity not found: {0}")]
    MissingEntity(String),
    #[error("node is not an entity: {0}")]
    NotEntity(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid field type: {0}")]
    InvalidField(String),
    #[error("action not found: {0}")]
    MissingAction(String),
    #[error("workflow not found: {0}")]
    MissingWorkflow(String),
    #[error("workflow step not found: {0}")]
    MissingStep(String),
}

impl RuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingEntity(_) => "ENTITY_NOT_FOUND",
            Self::NotEntity(_) => "NOT_ENTITY",
            Self::MissingField(_) => "MISSING_REQUIRED_FIELD",
            Self::InvalidField(_) => "INVALID_FIELD",
            Self::MissingAction(_) => "ACTION_NOT_FOUND",
            Self::MissingWorkflow(_) => "WORKFLOW_NOT_FOUND",
            Self::MissingStep(_) => "WORKFLOW_STEP_NOT_FOUND",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub id: String,
    pub values: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub id: u64,
    pub name: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Job {
    pub workflow: String,
    pub step: String,
    pub event_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeUpdate {
    pub subscription_id: u64,
    pub source: String,
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    pub id: u64,
    pub view: String,
    pub entity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentTask {
    pub id: u64,
    pub agent: String,
    pub workflow: String,
    pub event_id: u64,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Runtime {
    pub records: BTreeMap<String, Vec<Record>>,
    pub events: Vec<EventEnvelope>,
    pub jobs: VecDeque<Job>,
    pub agent_tasks: VecDeque<AgentTask>,

    #[serde(skip)]
    pub subscriptions: Vec<Subscription>,
    #[serde(skip)]
    pub updates: VecDeque<RealtimeUpdate>,

    next_record_id: u64,
    next_event_id: u64,
    next_subscription_id: u64,
    next_agent_task_id: u64,
}

impl Runtime {
    pub fn create(
        &mut self,
        app: &App,
        entity: &str,
        values: Map<String, Value>,
    ) -> Result<Record, RuntimeError> {
        let node = app
            .get(entity)
            .ok_or_else(|| RuntimeError::MissingEntity(entity.into()))?;
        let Node::Entity { fields, .. } = node else {
            return Err(RuntimeError::NotEntity(entity.into()));
        };

        validate_fields(fields, &values)?;

        self.next_record_id += 1;
        let record = Record {
            id: format!("r{}", self.next_record_id),
            values,
        };

        self.records
            .entry(entity.into())
            .or_default()
            .push(record.clone());

        self.publish(entity, "record.created", serde_json::to_value(&record).unwrap_or(Value::Null));
        Ok(record)
    }

    pub fn query(&self, entity: &str) -> Vec<Record> {
        self.records.get(entity).cloned().unwrap_or_default()
    }

    pub fn invoke_action(
        &mut self,
        app: &App,
        action: &str,
        values: Map<String, Value>,
    ) -> Result<(Option<Record>, Vec<EventEnvelope>), RuntimeError> {
        let (input, creates, emits) = match app.get(action) {
            Some(Node::Action {
                input,
                creates,
                emits,
                ..
            }) => (input.clone(), creates.clone(), emits.clone()),
            Some(_) => return Err(RuntimeError::MissingAction(action.into())),
            None => return Err(RuntimeError::MissingAction(action.into())),
        };

        validate_fields(&input, &values)?;

        let record = creates
            .first()
            .map(|entity| self.create(app, entity, values.clone()))
            .transpose()?;

        let payload = record
            .as_ref()
            .map(|r| serde_json::json!({"record_id": r.id, "values": r.values}))
            .unwrap_or_else(|| serde_json::json!({"values": values}));

        let mut emitted = Vec::new();
        for event in emits {
            emitted.push(self.emit(app, event, payload.clone()));
        }

        Ok((record, emitted))
    }

    pub fn emit(
        &mut self,
        app: &App,
        name: impl Into<String>,
        payload: Value,
    ) -> EventEnvelope {
        self.next_event_id += 1;
        let event = EventEnvelope {
            id: self.next_event_id,
            name: name.into(),
            payload,
        };
        self.events.push(event.clone());

        self.publish(&event.name, "event", serde_json::to_value(&event).unwrap_or(Value::Null));

        for node in app.nodes.values() {
            if let Node::Workflow {
                id,
                trigger,
                steps,
            } = node
            {
                if trigger == &event.name {
                    for step in steps {
                        self.jobs.push_back(Job {
                            workflow: id.clone(),
                            step: step.clone(),
                            event_id: event.id,
                        });
                    }
                }
            }
        }

        event
    }

    pub fn take_job(&mut self) -> Option<Job> {
        self.jobs.pop_front()
    }

    pub fn execute_next_job(&mut self, app: &App) -> Result<Option<JobResult>, RuntimeError> {
        let Some(job) = self.jobs.pop_front() else {
            return Ok(None);
        };

        let event = self
            .events
            .iter()
            .find(|event| event.id == job.event_id)
            .cloned()
            .ok_or_else(|| RuntimeError::WorkflowNotFound(job.workflow.clone()))?;

        let step = app
            .get(&job.step)
            .ok_or_else(|| RuntimeError::MissingStep(job.step.clone()))?;

        match step {
            Node::Action { .. } => {
                let values = event
                    .payload
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                let (record, events) = self.invoke_action(app, &job.step, values)?;
                Ok(Some(JobResult::Action {
                    job,
                    record,
                    events,
                }))
            }
            Node::Agent { .. } => {
                self.next_agent_task_id += 1;
                let task = AgentTask {
                    id: self.next_agent_task_id,
                    agent: job.step.clone(),
                    workflow: job.workflow.clone(),
                    event_id: event.id,
                    payload: event.payload,
                };
                self.agent_tasks.push_back(task.clone());
                Ok(Some(JobResult::AgentQueued { job, task }))
            }
            _ => Err(RuntimeError::MissingStep(job.step)),
        }
    }

    pub fn take_agent_task(&mut self) -> Option<AgentTask> {
        self.agent_tasks.pop_front()
    }

    pub fn subscribe(
        &mut self,
        app: &App,
        view: &str,
    ) -> Result<Subscription, RuntimeError> {
        let Some(Node::View { source, realtime, .. }) = app.get(view) else {
            return Err(RuntimeError::MissingStep(view.into()));
        };

        if !*realtime {
            return Err(RuntimeError::MissingStep(format!("{view} is not realtime")));
        }

        self.next_subscription_id += 1;
        let subscription = Subscription {
            id: self.next_subscription_id,
            view: view.into(),
            entity: source.clone(),
        };
        self.subscriptions.push(subscription.clone());

        Ok(subscription)
    }

    pub fn unsubscribe(&mut self, subscription_id: u64) -> bool {
        let before = self.subscriptions.len();
        self.subscriptions.retain(|s| s.id != subscription_id);
        before != self.subscriptions.len()
    }

    pub fn take_update(&mut self, subscription_id: u64) -> Option<RealtimeUpdate> {
        let index = self
            .updates
            .iter()
            .position(|update| update.subscription_id == subscription_id)?;
        self.updates.remove(index)
    }

    fn publish(&mut self, source: &str, kind: &str, payload: Value) {
        let matching: Vec<u64> = self
            .subscriptions
            .iter()
            .filter(|subscription| subscription.entity == source)
            .map(|subscription| subscription.id)
            .collect();

        for subscription_id in matching {
            self.updates.push_back(RealtimeUpdate {
                subscription_id,
                source: source.into(),
                kind: kind.into(),
                payload: payload.clone(),
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobResult {
    Action {
        job: Job,
        record: Option<Record>,
        events: Vec<EventEnvelope>,
    },
    AgentQueued {
        job: Job,
        task: AgentTask,
    },
}

fn validate_fields(
    fields: &BTreeMap<String, crate::Field>,
    values: &Map<String, Value>,
) -> Result<(), RuntimeError> {
    for (name, field) in fields {
        if field.required && !values.contains_key(name) {
            return Err(RuntimeError::MissingField(name.clone()));
        }
        if let Some(value) = values.get(name) {
            if !matches_type(&field.ty, value) {
                return Err(RuntimeError::InvalidField(name.clone()));
            }
        }
    }
    Ok(())
}

fn matches_type(ty: &FieldType, value: &Value) -> bool {
    match ty {
        FieldType::String | FieldType::Uuid | FieldType::Reference(_) => value.is_string(),
        FieldType::Integer => value.as_i64().is_some(),
        FieldType::Boolean => value.is_boolean(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, Node};

    fn ticket_app() -> App {
        let mut app = App::new("support");

        let mut fields = BTreeMap::new();
        fields.insert(
            "message".into(),
            Field {
                ty: FieldType::String,
                required: true,
            },
        );

        app.upsert(Node::Entity {
            id: "Ticket".into(),
            fields: fields.clone(),
        });

        let mut input = BTreeMap::new();
        input.insert(
            "message".into(),
            Field {
                ty: FieldType::String,
                required: true,
            },
        );

        app.upsert(Node::Action {
            id: "createTicket".into(),
            input,
            creates: vec!["Ticket".into()],
            emits: vec!["ticket.created".into()],
            requires_auth: false,
        });

        app.upsert(Node::Event {
            id: "ticket.created".into(),
        });

        app.upsert(Node::Agent {
            id: "supportAgent".into(),
            reads: vec!["Ticket".into()],
            writes: vec!["Ticket".into()],
        });

        app.upsert(Node::Workflow {
            id: "onTicketCreated".into(),
            trigger: "ticket.created".into(),
            steps: vec!["supportAgent".into()],
        });

        app.upsert(Node::View {
            id: "tickets".into(),
            source: "Ticket".into(),
            realtime: true,
        });

        app
    }

    #[test]
    fn creates_records_and_emits_workflow_jobs() {
        let app = ticket_app();
        let mut runtime = Runtime::default();
        let mut values = Map::new();
        values.insert("message".into(), Value::String("hello".into()));

        let record = runtime.create(&app, "Ticket", values).unwrap();
        assert_eq!(record.id, "r1");

        runtime.emit(
            &app,
            "ticket.created",
            serde_json::json!({"ticket":"r1"}),
        );
        assert_eq!(runtime.take_job().unwrap().step, "supportAgent");
    }

    #[test]
    fn action_creates_and_emits() {
        let app = ticket_app();
        let mut runtime = Runtime::default();
        let mut values = Map::new();
        values.insert("message".into(), Value::String("hello".into()));

        let (record, events) = runtime.invoke_action(&app, "createTicket", values).unwrap();
        assert_eq!(record.unwrap().id, "r1");
        assert_eq!(events[0].name, "ticket.created");
    }

    #[test]
    fn workflow_can_queue_agent_task() {
        let app = ticket_app();
        let mut runtime = Runtime::default();
        runtime.emit(
            &app,
            "ticket.created",
            serde_json::json!({"ticket":"r1"}),
        );

        let result = runtime.execute_next_job(&app).unwrap().unwrap();
        match result {
            JobResult::AgentQueued { task, .. } => assert_eq!(task.agent, "supportAgent"),
            other => panic!("unexpected result: {other:?}"),
        }

        assert_eq!(runtime.take_agent_task().unwrap().agent, "supportAgent");
    }

    #[test]
    fn realtime_subscription_receives_matching_updates() {
        let app = ticket_app();
        let mut runtime = Runtime::default();
        let subscription = runtime.subscribe(&app, "tickets").unwrap();

        let mut values = Map::new();
        values.insert("message".into(), Value::String("hello".into()));
        runtime.create(&app, "Ticket", values).unwrap();

        let update = runtime.take_update(subscription.id).unwrap();
        assert_eq!(update.source, "Ticket");
        assert_eq!(update.kind, "record.created");
    }
}
