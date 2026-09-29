pub mod engine;
pub mod protocol;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct App {
    pub name: String,
    pub nodes: BTreeMap<String, Node>,
}

impl App {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), nodes: BTreeMap::new() }
    }

    pub fn upsert(&mut self, node: Node) { self.nodes.insert(node.id().to_owned(), node); }

    pub fn get(&self, id: &str) -> Option<&Node> { self.nodes.get(id) }

    pub fn dependencies(&self, id: &str) -> BTreeSet<String> {
        self.nodes.get(id).map(Node::dependencies).unwrap_or_default()
    }

    pub fn dependents(&self, id: &str) -> BTreeSet<String> {
        self.nodes.values()
            .filter(|node| node.dependencies().contains(id))
            .map(|node| node.id().to_owned())
            .collect()
    }

    pub fn apply(&mut self, patch: Patch) {
        match patch.op {
            PatchOp::Upsert(node) => self.upsert(node),
            PatchOp::Remove { id } => { self.nodes.remove(&id); }
        }
    }

    pub fn to_json(&self) -> serde_json::Result<String> { serde_json::to_string_pretty(self) }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Node {
    Entity { id: String, fields: BTreeMap<String, Field> },
    Action { id: String, input: BTreeMap<String, Field>, creates: Vec<String>, emits: Vec<String>, requires_auth: bool },
    Event { id: String },
    Workflow { id: String, trigger: String, steps: Vec<String> },
    Agent { id: String, reads: Vec<String>, writes: Vec<String> },
    View { id: String, source: String, realtime: bool },
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Self::Entity { id, .. } | Self::Action { id, .. } | Self::Event { id }
            | Self::Workflow { id, .. } | Self::Agent { id, .. } | Self::View { id, .. } => id,
        }
    }

    pub fn dependencies(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        match self {
            Self::Entity { fields, .. } => {
                for field in fields.values() {
                    if let FieldType::Reference(target) = &field.ty { out.insert(target.clone()); }
                }
            }
            Self::Action { creates, emits, input, .. } => {
                out.extend(creates.iter().cloned());
                out.extend(emits.iter().cloned());
                out.extend(input.values().filter_map(|f| match &f.ty {
                    FieldType::Reference(target) => Some(target.clone()),
                    _ => None,
                }));
            }
            Self::Event { .. } => {}
            Self::Workflow { trigger, steps, .. } => {
                out.insert(trigger.clone());
                out.extend(steps.iter().cloned());
            }
            Self::Agent { reads, writes, .. } => {
                out.extend(reads.iter().cloned());
                out.extend(writes.iter().cloned());
            }
            Self::View { source, .. } => { out.insert(source.clone()); }
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub ty: FieldType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    String,
    Integer,
    Boolean,
    Uuid,
    Reference(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Patch { pub op: PatchOp }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatchOp {
    Upsert(Node),
    Remove { id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_tracks_dependencies_and_dependents() {
        let mut app = App::new("support");
        app.upsert(Node::Event { id: "ticket.created".into() });
        app.upsert(Node::Action {
            id: "createTicket".into(),
            input: BTreeMap::new(),
            creates: vec!["Ticket".into()],
            emits: vec!["ticket.created".into()],
            requires_auth: true,
        });
        app.upsert(Node::Workflow {
            id: "onTicketCreated".into(),
            trigger: "ticket.created".into(),
            steps: vec!["supportAgent".into()],
        });

        assert!(app.dependencies("createTicket").contains("Ticket"));
        assert!(app.dependents("ticket.created").contains("onTicketCreated"));
    }
}
