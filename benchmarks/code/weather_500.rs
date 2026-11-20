use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    for conn in listener.incoming() {
        let Ok(mut stream) = conn else { continue };
        let mut buf = [0_u8; 2048];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let path = req.split_whitespace().nth(1).unwrap_or("/");
        let path = path.split('?').next().unwrap_or("/");
        let (status, body) = match path {
            "/health" => ("200 OK", r#"{"status":"ok"}"#),
            "/weather" => ("500 INTERNAL SERVER ERROR", r#"{"error":"sensor offline"}"#),
            _ => ("404 NOT FOUND", r#"{"error":"not found"}"#),
        };
        let len = body.len();
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{body}"
        );
        let _ = stream.write_all(resp.as_bytes());
    }
}
