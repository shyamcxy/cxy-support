use onestack_core::{engine::{parse_oir, validate}, protocol::{handle, Request}, runtime::Runtime, App};
use std::{env, fs, io::{self, BufRead, Write}, path::PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("agent") {
        "import" => import_oir(args.get(2)),
        "agent" => run_agent(),
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: onestack [agent|import <file.oir>]");
            std::process::exit(2);
        }
    }
}

fn import_oir(file: Option<&String>) {
    let source = file.expect("usage: onestack import <file.oir>");
    let text = fs::read_to_string(source).expect("cannot read OIR file");
    let app = parse_oir(&text).expect("invalid OIR");
    let errors = validate(&app);
    if !errors.is_empty() {
        eprintln!("{}", serde_json::json!({"valid": false, "errors": errors}));
        std::process::exit(1);
    }
    save(&app);
    println!("{}", app.to_json().unwrap());
}

fn run_agent() {
    let mut app = load_app();
    let mut runtime = load_runtime();
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(v) => v,
            Err(e) => {
                writeln!(stdout, "{}", serde_json::json!({"id":null,"ok":false,"error":e.to_string()})).ok();
                continue;
            }
        };
        if line.trim().is_empty() { continue; }

        let request: Request = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                writeln!(stdout, "{}", serde_json::json!({"id":null,"ok":false,"error":format!("invalid request: {e}")})).ok();
                continue;
            }
        };

        let response = handle(&mut app, &mut runtime, request);
        if response.ok { save_app(&app); save_runtime(&runtime); }
        serde_json::to_writer(&mut stdout, &response).ok();
        writeln!(stdout).ok();
        stdout.flush().ok();
    }
}

fn state_path() -> PathBuf { PathBuf::from(".onestack/app.json") }

fn load() -> App {
    match fs::read_to_string(state_path()) {
        Ok(text) => serde_json::from_str(&text).expect("invalid .onestack/app.json"),
        Err(_) => App::new("agent-app"),
    }
}

fn save(app: &App) {
    let path = state_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).expect("cannot create .onestack"); }
    fs::write(path, app.to_json().expect("cannot serialize app")).expect("cannot save app");
}
