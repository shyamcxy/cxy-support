use crate::{engine::validate, runtime::Runtime, App, Node, Patch, PatchOp};
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
    pub patch: Option<Patch>,
    #[serde(default)]
    pub entity: Option<String>,
    #[serde(default)]
    pub data: Option<Map<String, Value>>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    pub id: Value,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn handle(app: &mut App, runtime: &mut Runtime, req: Request) -> Response {
    let id = req.id.clone();
    let result = match req.op.as_str() {
        "inspect" => req.node_id.as_deref()
            .and_then(|node_id| app.get(node_id))
            .map(|node| json!({
                "node": node,
                "dependencies": app.dependencies(node.id()),
                "dependents": app.dependents(node.id())
            }))
            .ok_or_else(|| "node not found".to_owned()),

        "list" => Ok(json!(app.nodes.keys().collect::<Vec<_>>())),

        "deps" => req.node_id.as_deref()
            .map(|node_id| json!({
                "dependencies": app.dependencies(node_id),
                "dependents": app.dependents(node_id)
            }))
            .ok_or_else(|| "node_id is required".to_owned()),

        "put" => req.node
            .map(|node| { let id = node.id().to_owned(); app.upsert(node); json!({"id": id}) })
            .ok_or_else(|| "node is required".to_owned()),

        "patch" => req.patch
            .map(|patch| { app.apply(patch); json!({"nodes": app.nodes.len()}) })
            .ok_or_else(|| "patch is required".to_owned()),

        "validate" => {
            let errors = validate(app);
            Ok(json!({ "valid": errors.is_empty(), "errors": errors }))
        }

        "create" => {
            let entity = req.entity.ok_or_else(|| "entity is required".to_owned())?;
            let data = req.data.unwrap_or_default();
            runtime.create(app, &entity, data)
                .map(|record| json!({"record": record}))
                .map_err(|e| e.to_string())
        }

        "query" => {
            let entity = req.entity.ok_or_else(|| "entity is required".to_owned())?;
            Ok(json!({"records": runtime.query(&entity)}))
        }

        "emit" => {
            let event = req.event.ok_or_else(|| "event is required".to_owned())?;
            let payload = req.payload.unwrap_or(Value::Null);
            let emitted = runtime.emit(app, event, payload);
            Ok(json!({"event": emitted, "queued_jobs": runtime.jobs.len()}))
        }

        "next_job" => Ok(json!({"job": runtime.take_job()})),

        "snapshot" => Ok(serde_json::to_value(&*app).unwrap_or(Value::Null)),

        "runtime_snapshot" => Ok(serde_json::to_value(&*runtime).unwrap_or(Value::Null)),

        _ => Err(format!("unknown operation: {}", req.op)),
    };

    match result {
        Ok(result) => Response { id, ok: true, result: Some(result), error: None },
        Err(error) => Response { id, ok: false, result: None, error: Some(error) },
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
    fn agent_can_mutate_and_inspect_graph() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();

        let request: Request = serde_json::from_value(json!({
            "id": 1,
            "op": "put",
            "node": {"kind": "Event", "id": "ticket.created"}
        })).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert!(response.ok);
        assert!(app.get("ticket.created").is_some());

        let request: Request = serde_json::from_value(json!({
            "id": 2,
            "op": "inspect",
            "node_id": "ticket.created"
        })).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert!(response.ok);
        assert_eq!(response.result.unwrap()["node"]["id"], "ticket.created");
    }

    #[test]
    fn agent_can_create_and_query_data() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("email".into(), Field { ty: FieldType::String, required: true });
        app.upsert(Node::Entity { id: "User".into(), fields });

        let request: Request = serde_json::from_value(json!({
            "id": 3,
            "op": "create",
            "entity": "User",
            "data": {"email": "a@example.com"}
        })).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert!(response.ok);

        let request: Request = serde_json::from_value(json!({
            "id": 4,
            "op": "query",
            "entity": "User"
        })).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert_eq!(response.result.unwrap()["records"][0]["values"]["email"], "a@example.com");
    }

    #[test]
    fn unknown_operation_is_structured_error() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();
        let request: Request = serde_json::from_value(json!({"id": "x", "op": "explode"})).unwrap();

        let response = handle(&mut app, &mut runtime, request);
        assert!(!response.ok);
        assert!(response.error.unwrap().contains("unknown operation"));
    }

    #[test]
    fn patch_operation_is_machine_friendly() {
        let mut app = App::new("test");
        let mut runtime = Runtime::default();
        let node = Node::Entity { id: "User".into(), fields: std::collections::BTreeMap::new() };
        let request = Request {
            id: json!(5),
            op: "patch".into(),
            node: None,
            node_id: None,
            patch: Some(patch_upsert(node)),
            entity: None,
            data: None,
            event: None,
            payload: None,
        };

        let response = handle(&mut app, &mut runtime, request);
        assert!(response.ok);
        assert!(app.get("User").is_some());
    }
}
