use crate::{engine::validate, App, Node, Patch, PatchOp};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

pub fn handle(app: &mut App, req: Request) -> Response {
    let id = req.id.clone();
    let result = match req.op.as_str() {
        "inspect" => req.node_id.as_deref()
            .and_then(|id| app.get(id))
            .map(|node| json!({
                "node": node,
                "dependencies": app.dependencies(node.id()),
                "dependents": app.dependents(node.id())
            }))
            .ok_or_else(|| "node not found".to_owned()),

        "list" => Ok(json!(app.nodes.keys().collect::<Vec<_>>())),

        "deps" => req.node_id.as_deref()
            .map(|id| json!({
                "dependencies": app.dependencies(id),
                "dependents": app.dependents(id)
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

        "snapshot" => Ok(serde_json::to_value(&*app).unwrap_or(Value::Null)),

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
