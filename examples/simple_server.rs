// simple_server.rs — простой HTTP сервер для U

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Запускает сервер на указанном адресе с обработчиком
pub fn serve(addr: &str, handler: fn(&str) -> String) {
    let listener = TcpListener::bind(addr).expect("Failed to bind");
    println!("Server listening on http://{}", addr);
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let handler = handler;
                thread::spawn(move || handle_client(stream, handler));
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}

fn handle_client(mut stream: TcpStream, handler: fn(&str) -> String) {
    let mut buffer = [0u8; 1024];
    if stream.read(&mut buffer).is_err() {
        return;
    }
    
    let request = String::from_utf8_lossy(&buffer);
    let path = parse_path(&request);
    
    let response_body = handler(path);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    
    let _ = stream.write_all(response.as_bytes());
}

fn parse_path(request: &str) -> &str {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return "/";
    }
    
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 {
        return "/";
    }
    
    parts[1]
}

/// Возвращает HTML ответ
pub fn html_response(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}

/// Возвращает JSON ответ  
pub fn json_response(content: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        content.len(),
        content
    )
}