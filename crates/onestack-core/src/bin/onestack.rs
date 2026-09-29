use onestack_core::{engine::{parse_oir, validate}, http, protocol::{handle, Request}, runtime::Runtime, App};
use std::{env, fs, io::{self, BufRead, Write}, path::PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("agent") {
        "import" => import_oir(args.get(2)),
        "agent" => run_agent(),
        "http" => run_http(args.get(2).map(String::as_str).unwrap_or("127.0.0.1:8787")),
        "bench" => {}
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("usage: onestack [agent|http [addr]|import <file.oir>]");
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
    save_app(&app);
    println!("{}", app.to_json().unwrap());
}

fn run_http(addr: &str) {
    let mut app = load_app();
    let mut runtime = load_runtime();

    eprintln!("OneStack HTTP listening on {addr}");
    http::serve(addr, &mut app, &mut runtime, |app, runtime| {
        save_app(app);
        save_runtime(runtime);
    })
    .expect("HTTP server failed");
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

fn app_path() -> PathBuf { PathBuf::from(".onestack/app.json") }
fn runtime_path() -> PathBuf { PathBuf::from(".onestack/runtime.json") }

fn load_app() -> App {
    match fs::read_to_string(app_path()) {
        Ok(text) => serde_json::from_str(&text).expect("invalid .onestack/app.json"),
        Err(_) => App::new("agent-app"),
    }
}

fn load_runtime() -> Runtime {
    match fs::read_to_string(runtime_path()) {
        Ok(text) => serde_json::from_str(&text).expect("invalid .onestack/runtime.json"),
        Err(_) => Runtime::default(),
    }
}

fn ensure_state_dir() {
    fs::create_dir_all(".onestack").expect("cannot create .onestack");
}

fn save_app(app: &App) {
    ensure_state_dir();
    fs::write(app_path(), app.to_json().expect("cannot serialize app")).expect("cannot save app");
}

fn save_runtime(runtime: &Runtime) {
    ensure_state_dir();
    fs::write(runtime_path(), serde_json::to_string_pretty(runtime).expect("cannot serialize runtime")).expect("cannot save runtime");
}
