use crate::{App, Field, FieldType, Node};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid OIR at line {line}: {message}")]
    Parse { line: usize, message: String },
}

pub fn parse_oir(source: &str) -> Result<App, EngineError> {
    let mut app: Option<App> = None;
    let mut lines = source.lines().enumerate().peekable();

    while let Some((idx, raw)) = lines.next() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let mut parts = line.split_whitespace();
        let head = parts.next().unwrap();

        match head {
            "APP" => {
                let name = parts.next().ok_or_else(|| EngineError::Parse { line: line_no, message: "APP needs a name".into() })?;
                app = Some(App::new(name));
            }
            "ENTITY" => {
                let id = required(parts.next(), line_no, "ENTITY needs an id")?;
                let mut fields = BTreeMap::new();
                while let Some((peek_idx, peek_raw)) = lines.peek().copied() {
                    let candidate = peek_raw.trim();
                    if candidate.is_empty() || candidate.starts_with('#') {
                        lines.next();
                        continue;
                    }
                    if candidate.split_whitespace().next().map(|x| matches!(x, "ENTITY"|"ACTION"|"EVENT"|"AGENT"|"WORKFLOW"|"VIEW"|"FILE"|"JOB")).unwrap_or(false) { break; }
                    let Some((name, rest)) = candidate.split_once(':') else { break; };
                    lines.next();
                    let mut p = rest.split_whitespace();
                    let ty = parse_type(&required(p.next(), peek_idx + 1, "field needs a type")?);
                    let required = p.any(|x| x == "required");
                    fields.insert(name.trim().to_owned(), Field { ty, required });
                }
                get_app(&mut app, line_no)?.upsert(Node::Entity { id, fields });
            }
            "ACTION" => {
                let id = required(parts.next(), line_no, "ACTION needs an id")?;
                let mut input = BTreeMap::new();
                let mut creates = Vec::new();
                let mut emits = Vec::new();
                let mut requires_auth = false;
                while let Some((peek_idx, peek_raw)) = lines.peek().copied() {
                    let candidate = peek_raw.trim();
                    if candidate.is_empty() || candidate.starts_with('#') { lines.next(); continue; }
                    let key = candidate.split_whitespace().next().unwrap_or("");
                    if matches!(key, "ENTITY"|"ACTION"|"EVENT"|"AGENT"|"WORKFLOW"|"VIEW"|"FILE"|"JOB") { break; }
                    if let Some(rest) = candidate.strip_prefix("input ") {
                        lines.next();
                        let (name, ty) = rest.split_once(':').ok_or_else(|| EngineError::Parse { line: peek_idx + 1, message: "input needs name:type".into() })?;
                        input.insert(name.trim().into(), Field { ty: parse_type(ty.trim()), required: true });
                    } else if let Some(rest) = candidate.strip_prefix("creates ") {
                        lines.next(); creates.push(rest.trim().into());
                    } else if let Some(rest) = candidate.strip_prefix("emits ") {
                        lines.next(); emits.push(rest.trim().into());
                    } else if candidate == "requires_auth true" {
                        lines.next(); requires_auth = true;
                    } else { break; }
                }
                get_app(&mut app, line_no)?.upsert(Node::Action { id, input, creates, emits, requires_auth });
            }
            "EVENT" => {
                let id = required(parts.next(), line_no, "EVENT needs an id")?;
                get_app(&mut app, line_no)?.upsert(Node::Event { id });
            }
            "AGENT" => {
                let id = required(parts.next(), line_no, "AGENT needs an id")?;
                let mut reads = Vec::new(); let mut writes = Vec::new();
                while let Some((_, peek_raw)) = lines.peek().copied() {
                    let candidate = peek_raw.trim();
                    if candidate.is_empty() { lines.next(); continue; }
                    let key = candidate.split_whitespace().next().unwrap_or("");
                    if matches!(key, "ENTITY"|"ACTION"|"EVENT"|"AGENT"|"WORKFLOW"|"VIEW") { break; }
                    if let Some(rest) = candidate.strip_prefix("reads ") { lines.next(); reads.push(rest.trim().into()); }
                    else if let Some(rest) = candidate.strip_prefix("writes ") { lines.next(); writes.push(rest.trim().into()); }
                    else { break; }
                }
                get_app(&mut app, line_no)?.upsert(Node::Agent { id, reads, writes });
            }
            "WORKFLOW" => {
                let id = required(parts.next(), line_no, "WORKFLOW needs an id")?;
                let mut trigger = String::new(); let mut steps = Vec::new();
                while let Some((_, peek_raw)) = lines.peek().copied() {
                    let candidate = peek_raw.trim();
                    if candidate.is_empty() { lines.next(); continue; }
                    let key = candidate.split_whitespace().next().unwrap_or("");
                    if matches!(key, "ENTITY"|"ACTION"|"EVENT"|"AGENT"|"WORKFLOW"|"VIEW") { break; }
                    if let Some(rest) = candidate.strip_prefix("trigger ") { lines.next(); trigger = rest.trim().into(); }
                    else if let Some(rest) = candidate.strip_prefix("steps ") { lines.next(); steps = rest.split(',').map(|x| x.trim().to_owned()).collect(); }
                    else { break; }
                }
                get_app(&mut app, line_no)?.upsert(Node::Workflow { id, trigger, steps });
            }
            "VIEW" => {
                let id = required(parts.next(), line_no, "VIEW needs an id")?;
                let mut source = String::new(); let mut realtime = false;
                while let Some((_, peek_raw)) = lines.peek().copied() {
                    let candidate = peek_raw.trim();
                    if candidate.is_empty() { lines.next(); continue; }
                    let key = candidate.split_whitespace().next().unwrap_or("");
                    if matches!(key, "ENTITY"|"ACTION"|"EVENT"|"AGENT"|"WORKFLOW"|"VIEW") { break; }
                    if let Some(rest) = candidate.strip_prefix("source ") { lines.next(); source = rest.trim().into(); }
                    else if candidate == "realtime true" { lines.next(); realtime = true; }
                    else { break; }
                }
                get_app(&mut app, line_no)?.upsert(Node::View { id, source, realtime });
            }
            other => return Err(EngineError::Parse { line: line_no, message: format!("unknown directive {other}") }),
        }
    }

    app.ok_or_else(|| EngineError::Parse { line: 1, message: "missing APP declaration".into() })
}

fn required(value: Option<&str>, line: usize, message: &str) -> Result<String, EngineError> {
    value.map(ToOwned::to_owned).ok_or_else(|| EngineError::Parse { line, message: message.into() })
}

fn get_app<'a>(app: &'a mut Option<App>, line: usize) -> Result<&'a mut App, EngineError> {
    app.as_mut().ok_or_else(|| EngineError::Parse { line, message: "declare APP before nodes".into() })
}

fn parse_type(raw: &str) -> FieldType {
    match raw {
        "String" => FieldType::String,
        "Integer" => FieldType::Integer,
        "Boolean" => FieldType::Boolean,
        "Uuid" => FieldType::Uuid,
        other if other.starts_with("Reference(") && other.ends_with(')') => {
            FieldType::Reference(other.trim_start_matches("Reference(").trim_end_matches(')').into())
        }
        _ => FieldType::String,
    }
}

pub fn validate(app: &App) -> Vec<String> {
    let mut errors = BTreeSet::new();
    for node in app.nodes.values() {
        for dep in node.dependencies() {
            if !app.nodes.contains_key(&dep) {
                errors.insert(format!("{} references missing node {}", node.id(), dep));
            }
        }
        if let Node::Workflow { trigger, .. } = node {
            if !matches!(app.get(trigger), Some(Node::Event { .. })) {
                errors.insert(format!("workflow {} trigger {} is not an Event", node.id(), trigger));
            }
        }
        if let Node::Job { timeout_ms, .. } = node {
            if *timeout_ms == 0 {
                errors.insert(format!("job {} timeout_ms must be > 0", node.id()));
            }
        }
        if let Node::View { source, .. } = node {
            if !matches!(app.get(source), Some(Node::Entity { .. }) | Some(Node::Job { .. })) {
                errors.insert(format!("view {} source {} is not an Entity or Job", node.id(), source));
            }
        }
    }
    errors.into_iter().collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_agent_oriented_oir() {
        let source = r#"
APP support

ENTITY User
  email:String required
  name:String required

ENTITY Ticket
  user:Reference(User) required
  message:String required
  status:String required

ACTION createTicket
  input message:String
  creates Ticket
  emits ticket.created
  requires_auth true

EVENT ticket.created

AGENT supportAgent
  reads Ticket
  writes Ticket

WORKFLOW onTicketCreated
  trigger ticket.created
  steps supportAgent

VIEW tickets
  source Ticket
  realtime true

FILE Video
  content_type video/mp4

EVENT video.completed

JOB generateVideo
  input prompt:String
  creates Video
  emits video.completed
  progress true
  timeout_ms 600000
  retries 2
"#;

        let app = parse_oir(source).expect("parse");
        assert_eq!(app.name, "support");
        assert_eq!(app.nodes.len(), 9);
        assert!(validate(&app).is_empty());
        assert!(app.dependencies("Ticket").contains("User"));
        assert!(app.dependencies("onTicketCreated").contains("ticket.created"));
    }

    #[test]
    fn reports_missing_dependencies() {
        let mut app = App::new("broken");
        app.upsert(Node::View { id: "tickets".into(), source: "Ticket".into(), realtime: true });
        let errors = validate(&app);
        assert_eq!(
            errors,
            vec![
                "tickets references missing node Ticket".to_string(),
                "view tickets source Ticket is not an Entity".to_string()
            ]
        );
    }
}
