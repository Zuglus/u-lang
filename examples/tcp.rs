// TCP OS boundary — всё что ниже U
// U видит только i64 (handles) и String
// Rust-типы (TcpListener, TcpStream) скрыты здесь

use tokio::net::{TcpListener, TcpStream};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

fn next_id() -> i64 { NEXT_ID.fetch_add(1, Ordering::SeqCst) }

static LISTENERS: std::sync::LazyLock<Mutex<HashMap<i64, TcpListener>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

static STREAMS: std::sync::LazyLock<Mutex<HashMap<i64, TcpStream>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn bind(port: i64) -> i64 {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    let id = next_id();
    LISTENERS.lock().await.insert(id, listener);
    id
}

pub async fn accept(listener_id: i64) -> i64 {
    let mut map = LISTENERS.lock().await;
    let listener = map.get_mut(&listener_id).unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    let stream_id = next_id();
    drop(map);
    STREAMS.lock().await.insert(stream_id, stream);
    stream_id
}

pub async fn tcp_write(stream_id: i64, data: String) {
    let mut map = STREAMS.lock().await;
    if let Some(stream) = map.get_mut(&stream_id) {
        stream.write_all(data.as_bytes()).await.unwrap();
    }
}
