use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

const ADDR: &str = "[::]:8080";

fn main() {
    let listener = TcpListener::bind(ADDR).expect("Failed to bind to IPv6 address");
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

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 3\r\n\r\nOK\n";
    let _ = stream.write_all(response.as_bytes());
}
