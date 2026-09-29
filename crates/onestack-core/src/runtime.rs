use crate::{App, FieldType, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, VecDeque};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
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
    #[error("project task not found: {0}")]
    TaskNotFound(u64),
    #[error("project task not ready: {0}")]
    TaskNotReady(u64),
    #[error("invalid task state: {0}")]
    InvalidTaskState(String),
    #[error("execution job not found: {0}")]
    MissingExecutionJob(u64),
    #[error("policy denied for {action} on {resource}")]
    PolicyDenied { action: String, resource: String },
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
            Self::TaskNotFound(_) => "TASK_NOT_FOUND",
            Self::TaskNotReady(_) => "TASK_NOT_READY",
            Self::InvalidTaskState(_) => "INVALID_TASK_STATE",
            Self::MissingExecutionJob(_) => "EXECUTION_JOB_NOT_FOUND",
            Self::PolicyDenied { .. } => "POLICY_DENIED",
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
pub struct WorkflowJob {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectTask {
    pub issue: u64,
    pub title: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LongJob {
    pub id: u64,
    pub kind: String,
    pub state: ExecutionState,
    pub progress: f32,
    pub input: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredFile {
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Runtime {
    pub records: BTreeMap<String, Vec<Record>>,
    pub events: Vec<EventEnvelope>,
    pub jobs: VecDeque<WorkflowJob>,
    pub agent_tasks: VecDeque<AgentTask>,
    #[serde(default)]
    pub project_tasks: BTreeMap<u64, ProjectTask>,
    #[serde(default)]
    pub execution_jobs: BTreeMap<u64, LongJob>,
    #[serde(default)]
    pub pending_execution_jobs: VecDeque<u64>,
    #[serde(default)]
    pub files: BTreeMap<String, StoredFile>,

    #[serde(skip)]
    pub subscriptions: Vec<Subscription>,
    #[serde(skip)]
    pub updates: VecDeque<RealtimeUpdate>,

    next_record_id: u64,
    next_event_id: u64,
    next_subscription_id: u64,
    next_agent_task_id: u64,
    next_execution_job_id: u64,
    next_file_id: u64,
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

    pub fn authorize(
        &self,
        app: &App,
        subject: &str,
        action: &str,
        resource: &str,
        context: &Map<String, Value>,
    ) -> bool {
        let requires_auth = matches!(
            app.get(action),
            Some(Node::Action { requires_auth: true, .. })
        );

        let policies: Vec<&crate::Node> = app.nodes.values().filter(|node| {
            let Node::Policy { subject: p_subject, action: p_action, resource: p_resource, .. } = node else {
                return false;
            };
            (p_subject == "*" || p_subject == subject)
                && (p_action == "*" || p_action == action)
                && (p_resource == "*" || p_resource == resource)
        }).collect();

        let mut matched_allow = false;
        for policy in policies {
            if let Node::Policy { allow, condition, .. } = policy {
                let condition_matches = match condition.as_deref() {
                    None | Some("*") => true,
                    Some(expr) => {
                        let Some((field, expected)) = expr.split_once('=') else { false };
                        context.get(field).map(|value| value.to_string().trim_matches('"') == expected).unwrap_or(false)
                    }
                };
                if condition_matches {
                    if !*allow {
                        return false;
                    }
                    matched_allow = true;
                }
            }
        }

        if matched_allow {
            true
        } else {
            !requires_auth
        }
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
                        self.jobs.push_back(WorkflowJob {
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

    pub fn take_job(&mut self) -> Option<WorkflowJob> {
        self.jobs.pop_front()
    }

    pub fn register_file(
        &mut self,
        name: String,
        content_type: String,
        size: u64,
        uri: String,
        kind: Option<String>,
    ) -> StoredFile {
        self.next_file_id += 1;
        let file = StoredFile {
            id: format!("file_{}", self.next_file_id),
            name,
            content_type,
            size,
            uri,
            kind,
        };
        self.files.insert(file.id.clone(), file.clone());
        file
    }

    pub fn get_file(&self, id: &str) -> Option<StoredFile> {
        self.files.get(id).cloned()
    }

    pub fn list_files(&self) -> Vec<StoredFile> {
        self.files.values().cloned().collect()
    }

    pub fn start_job(
        &mut self,
        app: &App,
        job_kind: &str,
        input: Map<String, Value>,
    ) -> Result<LongJob, RuntimeError> {
        let node = app
            .get(job_kind)
            .ok_or_else(|| RuntimeError::MissingStep(job_kind.into()))?;

        let Node::Job { input: schema, .. } = node else {
            return Err(RuntimeError::MissingStep(job_kind.into()));
        };

        validate_fields(schema, &input)?;

        self.next_execution_job_id += 1;
        let job = LongJob {
            id: self.next_execution_job_id,
            kind: job_kind.into(),
            state: ExecutionState::Queued,
            progress: 0.0,
            input,
            result: None,
            error: None,
        };

        self.execution_jobs.insert(job.id, job.clone());
        self.pending_execution_jobs.push_back(job.id);
        Ok(job)
    }

    pub fn take_execution_job(&mut self) -> Option<LongJob> {
        let id = self.pending_execution_jobs.pop_front()?;
        let job = self.execution_jobs.get_mut(&id)?;
        if job.state == ExecutionState::Queued {
            job.state = ExecutionState::Running;
        }
        Some(job.clone())
    }

    pub fn update_execution_job(
        &mut self,
        app: &App,
        id: u64,
        state: ExecutionState,
        progress: f32,
        result: Option<Value>,
        error: Option<String>,
    ) -> Result<LongJob, RuntimeError> {
        if !(0.0..=1.0).contains(&progress) {
            return Err(RuntimeError::InvalidTaskState(format!("progress must be between 0 and 1, got {progress}")));
        }

        let job_snapshot = {
            let job = self
                .execution_jobs
                .get_mut(&id)
                .ok_or(RuntimeError::MissingExecutionJob(id))?;

            job.state = state.clone();
            job.progress = progress;
            job.result = result.clone();
            job.error = error.clone();
            job.clone()
        };

        self.publish(
            &job_snapshot.kind,
            "job.updated",
            serde_json::to_value(&job_snapshot).unwrap_or(Value::Null),
        );

        if state == ExecutionState::Completed {
            if let Some(payload) = result {
                self.emit_job_events(app, &job_snapshot, payload);
            }
        }

        Ok(job_snapshot)
    }

    fn emit_job_events(&mut self, app: &App, job: &LongJob, payload: Value) {
        if let Some(Node::Job { emits, .. }) = app.get(&job.kind) {
            for event in emits.clone() {
                self.emit(app, event, payload.clone());
            }
        }
    }

    pub fn get_execution_job(&self, id: u64) -> Option<LongJob> {
        self.execution_jobs.get(&id).cloned()
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
            .ok_or_else(|| RuntimeError::MissingWorkflow(job.workflow.clone()))?;

        let step = app
            .get(&job.step)
            .ok_or_else(|| RuntimeError::MissingStep(job.step.clone()))?;

        match step {
            Node::Action { .. } => {
                let values = event
                    .payload
                    .get("values")
                    .and_then(Value::as_object)
                    .cloned()
                    .or_else(|| event.payload.as_object().cloned())
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

    pub fn seed_project_tasks(&mut self, tasks: Vec<ProjectTask>) {
        for task in tasks {
            self.project_tasks.entry(task.issue).or_insert(task);
        }
    }

    pub fn list_project_tasks(&self) -> Vec<ProjectTask> {
        self.project_tasks.values().cloned().collect()
    }

    pub fn next_project_task(&self) -> Option<ProjectTask> {
        self.project_tasks
            .values()
            .filter(|t| t.state == "READY")
            .min_by_key(|t| t.issue)
            .cloned()
    }

    pub fn claim_project_task(
        &mut self,
        issue: u64,
        agent: String,
    ) -> Result<ProjectTask, RuntimeError> {
        let task = self
            .project_tasks
            .get_mut(&issue)
            .ok_or(RuntimeError::TaskNotFound(issue))?;
        if task.state != "READY" {
            return Err(RuntimeError::TaskNotReady(issue));
        }
        task.state = "IN_PROGRESS".into();
        task.claimed_by = Some(agent);
        Ok(task.clone())
    }

    pub fn complete_project_task(
        &mut self,
        issue: u64,
        state: String,
        handoff: Option<String>,
    ) -> Result<ProjectTask, RuntimeError> {
        if state != "DONE" && state != "BLOCKED" {
            return Err(RuntimeError::InvalidTaskState(state));
        }
        let task = self
            .project_tasks
            .get_mut(&issue)
            .ok_or(RuntimeError::TaskNotFound(issue))?;
        if task.state != "IN_PROGRESS" {
            return Err(RuntimeError::TaskNotReady(issue));
        }
        task.state = state;
        task.handoff = handoff;
        Ok(task.clone())
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
        job: WorkflowJob,
        record: Option<Record>,
        events: Vec<EventEnvelope>,
    },
    AgentQueued {
        job: WorkflowJob,
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
    fn file_registration_is_runtime_native() {
        let mut runtime = Runtime::default();
        let file = runtime.register_file(
            "clip.mp4".into(),
            "video/mp4".into(),
            1024,
            "s3://bucket/clip.mp4".into(),
            Some("Video".into()),
        );
        assert_eq!(file.id, "file_1");
        assert_eq!(runtime.get_file("file_1").unwrap().size, 1024);
    }

    #[test]
    fn long_running_job_lifecycle_emits_on_completion() {
        let mut app = App::new("video");
        let mut input = BTreeMap::new();
        input.insert("prompt".into(), crate::Field { ty: FieldType::String, required: true });

        app.upsert(Node::File { id: "Video".into(), content_type: "video/mp4".into(), public: false });
        app.upsert(Node::Job {
            id: "generateVideo".into(),
            input,
            creates: vec!["Video".into()],
            emits: vec!["video.completed".into()],
            progress: true,
            timeout_ms: 600_000,
            retries: 2,
        });
        app.upsert(Node::Event { id: "video.completed".into() });

        let mut runtime = Runtime::default();
        let mut values = Map::new();
        values.insert("prompt".into(), Value::String("a cinematic desert".into()));

        let job = runtime.start_job(&app, "generateVideo", values).unwrap();
        assert_eq!(job.state, ExecutionState::Queued);

        let running = runtime.take_execution_job().unwrap();
        assert_eq!(running.state, ExecutionState::Running);

        let completed = runtime.update_execution_job(
            &app,
            running.id,
            ExecutionState::Completed,
            1.0,
            Some(serde_json::json!({"file_id":"f1"})),
            None,
        ).unwrap();
        assert_eq!(completed.state, ExecutionState::Completed);
        assert_eq!(runtime.events[0].name, "video.completed");
    }

    #[test]
    fn realtime_subscription_receives_job_progress() {
        let mut app = App::new("video");
        app.upsert(Node::Job {
            id: "generateVideo".into(),
            input: BTreeMap::new(),
            creates: vec![],
            emits: vec![],
            progress: true,
            timeout_ms: 600_000,
            retries: 1,
        });
        app.upsert(Node::View {
            id: "generations".into(),
            source: "generateVideo".into(),
            realtime: true,
        });

        let mut runtime = Runtime::default();
        let subscription = runtime.subscribe(&app, "generations").unwrap();
        let job = runtime.start_job(&app, "generateVideo", Map::new()).unwrap();
        let running = runtime.take_execution_job().unwrap();
        runtime.update_execution_job(&app, running.id, ExecutionState::Running, 0.4, None, None).unwrap();

        let update = runtime.take_update(subscription.id).unwrap();
        assert_eq!(update.kind, "job.updated");
        assert_eq!(update.source, "generateVideo");
        assert_eq!(update.payload["progress"], 0.4);
        assert_eq!(job.state, ExecutionState::Queued);
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

    #[test]
    fn project_task_claim_and_complete() {
        let mut runtime = Runtime::default();
        runtime.seed_project_tasks(vec![
            ProjectTask {
                issue: 2,
                title: "Durable Postgres execution backend".into(),
                state: "READY".into(),
                claimed_by: None,
                handoff: None,
            },
            ProjectTask {
                issue: 8,
                title: "Build an agent task picker and handoff endpoint".into(),
                state: "READY".into(),
                claimed_by: None,
                handoff: None,
            },
        ]);

        assert_eq!(runtime.next_project_task().unwrap().issue, 2);

        let claimed = runtime
            .claim_project_task(2, "agent-1".into())
            .unwrap();
        assert_eq!(claimed.state, "IN_PROGRESS");

        // Double-claim must fail with machine code.
        let err = runtime.claim_project_task(2, "agent-2".into()).unwrap_err();
        assert_eq!(err.code(), "TASK_NOT_READY");

        let done = runtime
            .complete_project_task(2, "DONE".into(), Some("seeded".into()))
            .unwrap();
        assert_eq!(done.state, "DONE");

        assert_eq!(runtime.next_project_task().unwrap().issue, 8);
    }

    #[test]
    fn project_task_rejects_bad_state() {
        let mut runtime = Runtime::default();
        runtime.seed_project_tasks(vec![ProjectTask {
            issue: 8,
            title: "picker".into(),
            state: "READY".into(),
            claimed_by: None,
            handoff: None,
        }]);
        runtime.claim_project_task(8, "agent-1".into()).unwrap();
        let err = runtime
            .complete_project_task(8, "IN_PROGRESS".into(), None)
            .unwrap_err();
        assert_eq!(err.code(), "INVALID_TASK_STATE");
    }
}
