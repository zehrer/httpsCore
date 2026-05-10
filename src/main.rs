use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::fs;

const ADDR: &str = "[::]:80";
const HTML_FILE: &str = "index.html";

fn main() {
    let listener = TcpListener::bind(ADDR).expect("Failed to bind — try: sudo setcap cap_net_bind_service=+ep ./server_runtime");
    println!("Listening on http://{ADDR}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream),
            Err(e) => eprintln!("Connection error: {e}"),
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let buf_reader = BufReader::new(&stream);

    let request_line = buf_reader
        .lines()
        .next()
        .and_then(|l| l.ok())
        .unwrap_or_default();

    println!("{peer} -> {request_line}");

    let (status, body) = match fs::read_to_string(HTML_FILE) {
        Ok(content) => ("200 OK", content),
        Err(_) => (
            "404 Not Found",
            "<html><body><h1>404 - index.html not found</h1></body></html>".to_string(),
        ),
    };

    let len = body.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {len}\r\n\r\n{body}"
    );
    let _ = stream.write_all(response.as_bytes());
}
