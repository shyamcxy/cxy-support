use onestack_core::{engine::parse_oir, protocol::{handle, Request}, runtime::Runtime};
use serde_json::json;
use std::time::Instant;

const EXAMPLE: &str = include_str!("../../../examples/support.oir");

fn main() {
    let app = parse_oir(EXAMPLE).expect("example OIR must parse");
    let full_context = serde_json::to_vec_pretty(&app).expect("serialize app");
    let node_id = "Ticket";
    let focused_context = serde_json::to_vec(&json!({
        "node_id": node_id,
        "node": app.get(node_id),
        "dependencies": app.dependencies(node_id),
        "dependents": app.dependents(node_id)
    }))
    .expect("serialize focused context");

    let mut runtime = Runtime::default();
    let start = Instant::now();
    let operations = [
        json!({"id":1,"op":"inspect","node_id":"Ticket"}),
        json!({"id":2,"op":"deps","node_id":"Ticket"}),
        json!({"id":3,"op":"validate"})
    ];

    for value in &operations {
        let request: Request = serde_json::from_value(value.clone()).expect("request");
        let response = handle(&mut app.clone(), &mut runtime, request);
        assert!(response.ok);
    }

    let elapsed_us = start.elapsed().as_micros();

    println!("{}", serde_json::json!({
        "benchmark":"onestack-agent-context",
        "application":"support",
        "metrics":{
            "full_snapshot_bytes": full_context.len(),
            "focused_context_bytes": focused_context.len(),
            "focused_context_ratio": focused_context.len() as f64 / full_context.len() as f64,
            "semantic_ops": operations.len(),
            "semantic_ops_elapsed_us": elapsed_us
        },
        "note":"This measures protocol/context overhead, not model tokens or conventional-framework end-to-end performance."
    }));
}
