use crate::{protocol::{handle, Request, Response}, runtime::Runtime, App};
use serde_json::json;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn serve<F>(
    addr: &str,
    app: &mut App,
    runtime: &mut Runtime,
    mut persist: F,
) -> io::Result<()>
where
    F: FnMut(&App, &Runtime),
{
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_connection(&mut stream, app, runtime, &mut persist) {
                    let _ = write_response(
                        &mut stream,
                        500,
                        &json!({"ok":false,"error":{"code":"HTTP_SERVER_ERROR","message":err.to_string()}}),
                    );
                }
            }
            Err(err) => {
                eprintln!("connection error: {err}");
            }
        }
    }
    Ok(())
}

fn handle_connection<F>(
    stream: &mut TcpStream,
    app: &mut App,
    runtime: &mut Runtime,
    persist: &mut F,
) -> io::Result<()>
where
    F: FnMut(&App, &Runtime),
{
    let mut buffer = Vec::new();
    let mut temp = [0_u8; 8192];

    let header_end;
    loop {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&temp[..read]);
        if let Some(position) = find_header_end(&buffer) {
            header_end = position;
            break;
        }
        if buffer.len() > 64 * 1024 {
            return write_response(
                stream,
                413,
                &json!({"ok":false,"error":{"code":"HTTP_HEADERS_TOO_LARGE","message":"headers too large"}}),
            );
        }
    }

    let (method, path, content_length): (String, String, usize) = {
        let header_bytes = &buffer[..header_end];
        let header_text = String::from_utf8_lossy(header_bytes);
        let mut lines = header_text.lines();
        let request_line = lines.next().unwrap_or_default().to_owned();

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_owned();
        let path = parts.next().unwrap_or_default().to_owned();

        let mut content_length = 0usize;
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        (method, path, content_length)
    };

    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let read = stream.read(&mut temp)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&temp[..read]);
    }

    if method == "GET" && path == "/health" {
        return write_response(
            stream,
            200,
            &json!({"ok":true,"service":"onestack","version":"0.1"}),
        );
    }

    if method != "POST" || path != "/agent" {
        return write_response(
            stream,
            404,
            &json!({"ok":false,"error":{"code":"HTTP_NOT_FOUND","message":"use POST /agent or GET /health"}}),
        );
    }

    let body_end = body_start.saturating_add(content_length).min(buffer.len());
    let body = &buffer[body_start..body_end];
    let request: Request = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(err) => {
            return write_response(
                stream,
                400,
                &json!({"ok":false,"error":{"code":"INVALID_JSON","message":err.to_string()}}),
            )
        }
    };

    let should_persist = request.op != "snapshot" && request.op != "runtime_snapshot";
    let response: Response = handle(app, runtime, request);
    if response.ok && should_persist {
        persist(app, runtime);
    }

    let status = if response.ok { 200 } else { 400 };
    write_response(stream, status, &response)
}

fn write_response<T: serde::Serialize>(
    stream: &mut TcpStream,
    status: u16,
    body: &T,
) -> io::Result<()> {
    let bytes = serde_json::to_vec(body).unwrap_or_else(|_| br#"{"ok":false}"#.to_vec());
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };

    write!(
        stream,
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    )?;
    stream.write_all(&bytes)?;
    stream.flush()
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_http_header_end() {
        let data = b"POST /agent HTTP/1.1\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(find_header_end(data), Some(39));
    }
}
