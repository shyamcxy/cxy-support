use crate::{
    engine::validate,
    runtime::{Runtime, RuntimeError},
    App, Node, Patch, PatchOp,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: Value,
    pub op: String,
    #[serde(default)]
    pub node: Option<Node>,
    #[serde(default)]
    pub node_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub patch: Option<Patch>,
    #[serde(default)]
    pub entity: Option<String>,
    #[serde(default)]
    pub data: Option<Map<String, Value>>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
    #[serde(default)]
    pub view: Option<String>,
    #[serde(default)]
    pub subscription_id: Option<u64>,
    #[serde(default)]
    pub issue: Option<u64>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub task_state: Option<String>,
    #[serde(default)]
    pub handoff: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub id: Value,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
}

fn error(
    req: &Request,
    code: impl Into<String>,
    message: impl Into<String>,
) -> ErrorPayload {
    ErrorPayload {
        code: code.into(),
        message: message.into(),
        operation: Some(req.op.clone()),
        node_id: req.node_id.clone(),
        field: None,
    }
}

fn runtime_error(req: &Request, err: RuntimeError) -> ErrorPayload {
    ErrorPayload {
        code: err.code().to_owned(),
        message: err.to_string(),
        operation: Some(req.op.clone()),
        node_id: req
            .node_id
            .clone()
            .or_else(|| req.action.clone())
            .or_else(|| req.issue.map(|i| i.to_string())),
        field: match err {
            RuntimeError::MissingField(field) | RuntimeError::InvalidField(field) => Some(field),
            _ => None,
        },
    }
}

pub fn handle(app: &mut App, runtime: &mut Runtime, req: Request) -> Response {
    let id = req.id.clone();

    let result: Result<Value, ErrorPayload> = match req.op.as_str() {
        "inspect" => req
            .node_id
            .as_deref()
            .and_then(|node_id| app.get(node_id))
            .map(|node| {
                json!({
                    "node": node,
                    "dependencies": app.dependencies(node.id()),
                    "dependents": app.dependents(node.id())
                })
            })
            .ok_or_else(|| error(&req, "NODE_NOT_FOUND", "node was not found")),

        "list" => Ok(json!(app.nodes.keys().collect::<Vec<_>>())),

        "deps" => req
            .node_id
            .as_deref()
            .map(|node_id| {
                json!({
                    "dependencies": app.dependencies(node_id),
                    "dependents": app.dependents(node_id)
                })
            })
            .ok_or_else(|| error(&req, "MISSING_NODE_ID", "node_id is required")),

        "put" => req
            .node
            .clone()
            .map(|node| {
                let id = node.id().to_owned();
                app.upsert(node);
                json!({"id": id})
            })
            .ok_or_else(|| error(&req, "MISSING_NODE", "node is required")),

        "patch" => req
            .patch
            .clone()
            .map(|patch| {
                app.apply(patch);
                json!({"nodes": app.nodes.len()})
            })
            .ok_or_else(|| error(&req, "MISSING_PATCH", "patch is required")),

        "validate" => {
            let errors = validate(app);
            Ok(json!({
                "valid": errors.is_empty(),
                "errors": errors
            }))
        }

        "invoke" => {
            let action = req
                .action
                .clone()
                .ok_or_else(|| error(&req, "MISSING_ACTION", "action is required"));
            match action {
                Ok(action) => {
                    let data = req.data.clone().unwrap_or_default();
                    runtime
                        .invoke_action(app, &action, data)
                        .map(|(record, events)| {
                            json!({
                                "record": record,
                                "events": events,
                                "queued_jobs": runtime.jobs.len()
                            })
                        })
                        .map_err(|err| runtime_error(&req, err))
                }
                Err(err) => Err(err),
            }
        }

        "create" => {
            let entity = req
                .entity
                .clone()
                .ok_or_else(|| error(&req, "MISSING_ENTITY", "entity is required"));
            match entity {
                Ok(entity) => runtime
                    .create(app, &entity, req.data.clone().unwrap_or_default())
                    .map(|record| json!({"record": record}))
                    .map_err(|err| runtime_error(&req, err)),
                Err(err) => Err(err),
            }
        }

        "query" => {
            let entity = req
                .entity
                .clone()
                .ok_or_else(|| error(&req, "MISSING_ENTITY", "entity is required"));
            match entity {
                Ok(entity) => Ok(json!({"records": runtime.query(&entity)})),
                Err(err) => Err(err),
            }
        }

        "emit" => {
            let event = req
                .event
                .clone()
                .ok_or_else(|| error(&req, "MISSING_EVENT", "event is required"));
            match event {
                Ok(event) => Ok(json!({
                    "event": runtime.emit(app, event, req.payload.clone().unwrap_or(Value::Null)),
                    "queued_jobs": runtime.jobs.len()
                })),
                Err(err) => Err(err),
            }
        }

        "next_job" => Ok(json!({"job": runtime.take_job()})),

        "execute_job" => runtime
            .execute_next_job(app)
            .map(|result| json!({"result": result, "queued_jobs": runtime.jobs.len()}))
            .map_err(|err| runtime_error(&req, err)),

        "next_agent_task" => Ok(json!({"task": runtime.take_agent_task()})),

        "list_tasks" => Ok(json!({"tasks": runtime.list_project_tasks()})),

        "next_task" => Ok(json!({"task": runtime.next_project_task()})),

        "seed_tasks" => {
            let tasks = req
                .payload
                .clone()
                .and_then(|p| serde_json::from_value::<Vec<crate::runtime::ProjectTask>>(p).ok())
                .ok_or_else(|| error(&req, "MISSING_TASKS", "payload must be an array of {issue,title,state}"));
            match tasks {
                Ok(tasks) => {
                    runtime.seed_project_tasks(tasks);
                    Ok(json!({"tasks": runtime.list_project_tasks()}))
                }
                Err(err) => Err(err),
            }
        }

        "claim_task" => {
            let issue = req
                .issue
                .ok_or_else(|| error(&req, "MISSING_ISSUE", "issue is required"));
            let agent = req
                .agent
                .clone()
                .ok_or_else(|| error(&req, "MISSING_AGENT", "agent is required"));
            match (issue, agent) {
                (Ok(issue), Ok(agent)) => runtime
                    .claim_project_task(issue, agent)
                    .map(|task| json!({"task": task}))
                    .map_err(|err| runtime_error(&req, err)),
                (Err(err), _) | (_, Err(err)) => Err(err),
            }
        }

        "complete_task" => {
            let issue = req
                .issue
                .ok_or_else(|| error(&req, "MISSING_ISSUE", "issue is required"));
            let state = req
                .task_state
                .clone()
                .ok_or_else(|| error(&req, "MISSING_TASK_STATE", "task_state must be DONE or BLOCKED"));
            match (issue, state) {
                (Ok(issue), Ok(state)) => runtime
                    .complete_project_task(issue, state, req.handoff.clone())
                    .map(|task| json!({"task": task}))
                    .map_err(|err| runtime_error(&req, err)),
                (Err(err), _) | (_, Err(err)) => Err(err),
            }
        }

        "subscribe" => {
            let view = req
                .view
                .clone()
                .ok_or_else(|| error(&req, "MISSING_VIEW", "view is required"));
            match view {
                Ok(view) => runtime
                    .subscribe(app, &view)
                    .map(|subscription| json!({"subscription": subscription}))
                    .map_err(|err| runtime_error(&req, err)),
                Err(err) => Err(err),
            }
        }

        "unsubscribe" => {
            let id = req
                .subscription_id
                .ok_or_else(|| error(&req, "MISSING_SUBSCRIPTION", "subscription_id is required"));
            match id {
                Ok(id) => Ok(json!({"removed": runtime.unsubscribe(id)})),
                Err(err) => Err(err),
            }
        }

        "next_update" => {
            let id = req
                .subscription_id
                .ok_or_else(|| error(&req, "MISSING_SUBSCRIPTION", "subscription_id is required"));
            match id {
                Ok(id) => Ok(json!({"update": runtime.take_update(id)})),
                Err(err) => Err(err),
            }
        }

        "snapshot" => Ok(serde_json::to_value(&*app).unwrap_or(Value::Null)),
        "runtime_snapshot" => Ok(serde_json::to_value(&*runtime).unwrap_or(Value::Null)),

        _ => Err(error(&req, "UNKNOWN_OPERATION", "operation is not supported")),
    };

    match result {
        Ok(result) => Response {
            id,
            ok: true,
            result: Some(result),
            error: None,
        },
        Err(error) => Response {
            id,
            ok: false,
            result: None,
            error: Some(error),
        },
    }
}

pub fn patch_upsert(node: Node) -> Patch {
    Patch { op: PatchOp::Upsert(node) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, FieldType};

    #[test]
    fn unknown_operation_has_machine_code() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();

        let request: Request =
            serde_json::from_value(json!({"id": "x", "op": "explode"})).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "UNKNOWN_OPERATION");
    }

    #[test]
    fn missing_field_has_machine_code_and_field() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();

        let mut fields = std::collections::BTreeMap::new();
        fields.insert(
            "email".into(),
            Field {
                ty: FieldType::String,
                required: true,
            },
        );
        app.upsert(Node::Entity {
            id: "User".into(),
            fields,
        });

        let request: Request = serde_json::from_value(json!({
            "id": 1,
            "op": "create",
            "entity": "User",
            "data": {}
        }))
        .unwrap();

        let response = handle(&mut app, &mut runtime, request);
        let err = response.error.unwrap();
        assert_eq!(err.code, "MISSING_REQUIRED_FIELD");
        assert_eq!(err.field.as_deref(), Some("email"));
    }

    #[test]
    fn realtime_protocol_round_trip() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();

        let mut fields = std::collections::BTreeMap::new();
        fields.insert(
            "message".into(),
            Field {
                ty: FieldType::String,
                required: true,
            },
        );

        app.upsert(Node::Entity {
            id: "Ticket".into(),
            fields,
        });
        app.upsert(Node::View {
            id: "tickets".into(),
            source: "Ticket".into(),
            realtime: true,
        });

        let response = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({
                "id": 1,
                "op": "subscribe",
                "view": "tickets"
            }))
            .unwrap(),
        );

        let subscription_id = response.result.unwrap()["subscription"]["id"]
            .as_u64()
            .unwrap();

        runtime
            .create(
                &app,
                "Ticket",
                serde_json::from_value(json!({"message":"hello"})).unwrap(),
            )
            .unwrap();

        let update = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({
                "id": 2,
                "op": "next_update",
                "subscription_id": subscription_id
            }))
            .unwrap(),
        );

        assert_eq!(update.result.unwrap()["update"]["source"], "Ticket");
    }

    #[test]
    fn patch_operation_is_machine_friendly() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();
        let node = Node::Entity {
            id: "User".into(),
            fields: std::collections::BTreeMap::new(),
        };
        let request = Request {
            id: json!(5),
            op: "patch".into(),
            node: None,
            node_id: None,
            action: None,
            patch: Some(patch_upsert(node)),
            entity: None,
            data: None,
            event: None,
            payload: None,
            view: None,
            subscription_id: None,
            issue: None,
            agent: None,
            task_state: None,
            handoff: None,
            title: None,
        };

        let response = handle(&mut app, &mut runtime, request);
        assert!(response.ok);
        assert!(app.get("User").is_some());
    }

    #[test]
    fn task_picker_round_trip() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();

        let seed = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({
                "id": 1,
                "op": "seed_tasks",
                "payload": [
                    {"issue": 2, "title": "Durable Postgres execution backend", "state": "READY"},
                    {"issue": 8, "title": "Build an agent task picker and handoff endpoint", "state": "READY"}
                ]
            }))
            .unwrap(),
        );
        assert!(seed.ok);

        let next = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({"id": 2, "op": "next_task"})).unwrap(),
        );
        assert_eq!(next.result.unwrap()["task"]["issue"], 2);

        let claim = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({"id": 3, "op": "claim_task", "issue": 2, "agent": "agent-1"}))
                .unwrap(),
        );
        assert!(claim.ok);

        let done = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({"id": 4, "op": "complete_task", "issue": 2, "task_state": "DONE", "handoff": "seeded"}))
                .unwrap(),
        );
        assert!(done.ok);
        assert_eq!(done.result.unwrap()["task"]["state"], "DONE");

        let next2 = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({"id": 5, "op": "next_task"})).unwrap(),
        );
        assert_eq!(next2.result.unwrap()["task"]["issue"], 8);
    }

    #[test]
    fn claim_missing_task_has_machine_code() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();
        let response = handle(
            &mut app,
            &mut runtime,
            serde_json::from_value(json!({"id": 1, "op": "claim_task", "issue": 99, "agent": "a"}))
                .unwrap(),
        );
        assert!(!response.ok);
        assert_eq!(response.error.unwrap().code, "TASK_NOT_FOUND");
    }
}
