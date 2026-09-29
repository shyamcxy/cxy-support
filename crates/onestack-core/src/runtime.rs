use crate::{App, FieldType, Node};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, VecDeque};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("entity not found: {0}")]
    MissingEntity(String),
    #[error("node is not an entity: {0}")]
    NotEntity(String),
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid field type: {0}")]
    InvalidField(String),
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Runtime {
    pub records: BTreeMap<String, Vec<Record>>,
    pub events: Vec<EventEnvelope>,
    pub jobs: VecDeque<Job>,
    next_record_id: u64,
    next_event_id: u64,
}

impl Runtime {
    pub fn create(&mut self, app: &App, entity: &str, values: Map<String, Value>) -> Result<Record, RuntimeError> {
        let node = app.get(entity).ok_or_else(|| RuntimeError::MissingEntity(entity.into()))?;
        let Node::Entity { fields, .. } = node else { return Err(RuntimeError::NotEntity(entity.into())); };

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

        self.next_record_id += 1;
        let record = Record {
            id: format!("r{}", self.next_record_id),
            values,
        };
        self.records.entry(entity.into()).or_default().push(record.clone());
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
            Some(Node::Action { input, creates, emits, .. }) => (input.clone(), creates.clone(), emits.clone()),
            Some(_) => return Err(RuntimeError::NotEntity(action.into())),
            None => return Err(RuntimeError::MissingEntity(action.into())),
        };

        for (name, field) in &input {
            if field.required && !values.contains_key(name) {
                return Err(RuntimeError::MissingField(name.clone()));
            }
            if let Some(value) = values.get(name) {
                if !matches_type(&field.ty, value) {
                    return Err(RuntimeError::InvalidField(name.clone()));
                }
            }
        }

        let record = creates.first()
            .map(|entity| self.create(app, entity, values.clone()))
            .transpose()?;

        let payload = record.as_ref()
            .map(|r| serde_json::json!({"record_id": r.id, "values": r.values}))
            .unwrap_or_else(|| serde_json::json!({"values": values}));

        let mut emitted = Vec::new();
        for event in emits {
            emitted.push(self.emit(app, event, payload.clone()));
        }

        Ok((record, emitted))
    }

    pub fn emit(&mut self, app: &App, name: impl Into<String>, payload: Value) -> EventEnvelope {
        self.next_event_id += 1;
        let event = EventEnvelope { id: self.next_event_id, name: name.into(), payload };
        self.events.push(event.clone());

        for node in app.nodes.values() {
            if let Node::Workflow { id, trigger, steps } = node {
                if trigger == &event.name {
                    for step in steps {
                        self.jobs.push_back(Job { workflow: id.clone(), step: step.clone(), event_id: event.id });
                    }
                }
            }
        }

        event
    }

    pub fn take_job(&mut self) -> Option<Job> { self.jobs.pop_front() }
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

    #[test]
    fn creates_records_and_emits_workflow_jobs() {
        let mut app = App::new("support");
        let mut fields = BTreeMap::new();
        fields.insert("message".into(), Field { ty: FieldType::String, required: true });
        app.upsert(Node::Entity { id: "Ticket".into(), fields });
        app.upsert(Node::Event { id: "ticket.created".into() });
        app.upsert(Node::Agent { id: "supportAgent".into(), reads: vec!["Ticket".into()], writes: vec!["Ticket".into()] });
        app.upsert(Node::Workflow { id: "onTicketCreated".into(), trigger: "ticket.created".into(), steps: vec!["supportAgent".into()] });

        let mut runtime = Runtime::default();
        let mut values = Map::new();
        values.insert("message".into(), Value::String("hello".into()));

        let record = runtime.create(&app, "Ticket", values).expect("record");
        assert_eq!(record.id, "r1");

        runtime.emit(&app, "ticket.created", serde_json::json!({"ticket":"r1"}));
        assert_eq!(runtime.take_job().unwrap().step, "supportAgent");
    }
}
