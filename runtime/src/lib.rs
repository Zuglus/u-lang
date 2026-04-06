//! U Language Runtime
//!
//! Provides Sqlite, Args, concurrency (tokio), HTTP, and string extensions for U scripts.

use std::error::Error;

// Re-export everything at crate root for `use u_runtime::*;`
#[cfg(feature = "sqlite")]
pub use db::{Db, Row, Sqlite};
pub use args::{Args, ParsedArgs};
pub use concurrency::{Channel, Chan};
#[cfg(feature = "http")]
pub use http::{HttpServer, HttpListener, HttpConn, Response, HttpResponse, HttpRequest, Router, URouter, serve};

/// Extension trait: .int() on strings
pub trait StrExt {
    fn int(&self) -> Result<i64, Box<dyn Error + Send + Sync>>;
}

impl StrExt for String {
    fn int(&self) -> Result<i64, Box<dyn Error + Send + Sync>> {
        Ok(self.parse::<i64>()?)
    }
}

impl StrExt for str {
    fn int(&self) -> Result<i64, Box<dyn Error + Send + Sync>> {
        Ok(self.parse::<i64>()?)
    }
}

/// Parse string to i64, return Option
pub fn str_to_int(s: &str) -> Option<i64> {
    s.parse().ok()
}

/// Parse string to f64, return Option
pub fn str_to_float(s: &str) -> Option<f64> {
    s.parse().ok()
}

// ─── Sqlite ──────────────────────────────────────────────

#[cfg(feature = "sqlite")]
pub mod db {
    use super::*;
    use std::collections::HashMap;
    use rusqlite::Connection;

    /// Unit struct — used as `Sqlite.open("path")` in U source
    pub struct Sqlite;

    impl Sqlite {
        pub fn open(&self, path: &str) -> Result<Db, Box<dyn Error + Send + Sync>> {
            let conn = Connection::open(path)?;
            Ok(Db { conn })
        }
    }

    pub struct Db {
        conn: Connection,
    }

    #[derive(Debug, Clone)]
    pub enum Value {
        Int(i64),
        Float(f64),
        Text(String),
        Bool(bool),
        Null,
    }

    #[derive(Debug, Clone)]
    pub struct Row {
        data: HashMap<String, Value>,
    }

    impl Db {
        /// Execute without params
        pub fn exec(&self, sql: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
            let sql = convert_params(sql);
            self.conn.execute(&sql, ())?;
            Ok(())
        }

        /// Execute with one param (any type implementing ToSql)
        pub fn exec1<T: rusqlite::types::ToSql>(&self, sql: &str, param: &T) -> Result<(), Box<dyn Error + Send + Sync>> {
            let sql = convert_params(sql);
            self.conn.execute(&sql, [param as &dyn rusqlite::types::ToSql])?;
            Ok(())
        }

        /// Query without params
        pub fn query(&self, sql: &str) -> Result<Vec<Row>, Box<dyn Error + Send + Sync>> {
            self.query_internal(&convert_params(sql), ())
        }

        /// Query with one param
        pub fn query1<T: rusqlite::types::ToSql>(&self, sql: &str, param: &T) -> Result<Vec<Row>, Box<dyn Error + Send + Sync>> {
            self.query_internal(&convert_params(sql), [param as &dyn rusqlite::types::ToSql])
        }

        fn query_internal<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<Row>, Box<dyn Error + Send + Sync>> {
            let mut stmt = self.conn.prepare(sql)?;
            let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let rows = stmt.query_map(params, |row| {
                let mut data = HashMap::new();
                for (i, name) in col_names.iter().enumerate() {
                    let val = match row.get_ref(i)? {
                        rusqlite::types::ValueRef::Integer(n) => Value::Int(n),
                        rusqlite::types::ValueRef::Real(f) => Value::Float(f),
                        rusqlite::types::ValueRef::Text(s) => {
                            Value::Text(String::from_utf8_lossy(s).into_owned())
                        }
                        rusqlite::types::ValueRef::Blob(_) => Value::Null,
                        rusqlite::types::ValueRef::Null => Value::Null,
                    };
                    data.insert(name.clone(), val);
                }
                Ok(Row { data })
            })?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        }
    }

    impl Row {
        pub fn int(&self, col: &str) -> i64 {
            match self.data.get(col) {
                Some(Value::Int(n)) => *n,
                Some(Value::Float(f)) => *f as i64,
                Some(Value::Text(s)) => s.parse().unwrap_or(0),
                _ => 0,
            }
        }

        pub fn string(&self, col: &str) -> String {
            match self.data.get(col) {
                Some(Value::Text(s)) => s.clone(),
                Some(Value::Int(n)) => n.to_string(),
                Some(Value::Float(f)) => f.to_string(),
                _ => String::new(),
            }
        }

        pub fn bool(&self, col: &str) -> bool {
            match self.data.get(col) {
                Some(Value::Int(n)) => *n != 0,
                Some(Value::Bool(b)) => *b,
                Some(Value::Text(s)) => s == "true" || s == "1",
                _ => false,
            }
        }
    }

    fn convert_params(sql: &str) -> String {
        let mut result = sql.to_string();
        for i in (1..=9).rev() {
            result = result.replace(&format!("${}", i), &format!("?{}", i));
        }
        result
    }
}

// ─── Args ────────────────────────────────────────────────

pub mod args {
    use super::*;

    /// Unit struct — used as `Args.parse()` in U source
    pub struct Args;

    impl Args {
        pub fn parse(&self) -> ParsedArgs {
            let all: Vec<String> = std::env::args().collect();
            let command = all.get(1).cloned().unwrap_or_default();
            let positional: Vec<String> = if all.len() > 2 {
                all[2..].to_vec()
            } else {
                vec![]
            };
            ParsedArgs { command, positional }
        }
    }

    pub struct ParsedArgs {
        pub command: String,
        positional: Vec<String>,
    }

    impl ParsedArgs {
        pub fn require(&self, _name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
            if self.positional.is_empty() {
                return Err(format!("Отсутствует аргумент: <{}>", _name).into());
            }
            Ok(self.positional.join(" "))
        }
    }
}

// ─── Concurrency (tokio) ─────────────────────────────────

pub mod concurrency {
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    /// Unit struct — used as `Channel.new()` in U source
    pub struct Channel;

    impl Channel {
        pub fn new(&self) -> Chan {
            let (tx, rx) = mpsc::unbounded_channel();
            Chan { tx, rx: Arc::new(Mutex::new(rx)) }
        }
    }

    /// The actual channel value, cloneable across tasks
    #[derive(Clone)]
    pub struct Chan {
        tx: mpsc::UnboundedSender<String>,
        rx: Arc<Mutex<mpsc::UnboundedReceiver<String>>>,
    }

    impl Chan {
        pub fn send(&self, msg: &str) {
            let _ = self.tx.send(msg.to_string());
        }

        pub async fn recv(&self) -> String {
            self.rx.lock().await.recv().await.unwrap_or_default()
        }

        pub async fn recv_timeout(&self, ms: i64) -> Option<String> {
            let timeout = tokio::time::Duration::from_millis(ms as u64);
            match tokio::time::timeout(timeout, self.rx.lock().await.recv()).await {
                Ok(Some(msg)) => Some(msg),
                _ => None,
            }
        }
    }
}

/// Sleep for given milliseconds (async, yields to tokio runtime)
pub async fn sleep(ms: i64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
}

/// Read file contents as String
pub fn read_file(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    Ok(std::fs::read_to_string(path)?)
}

/// Determine MIME type from file extension
pub fn mime_type(path: &str) -> String {
    if path.ends_with(".html") || path.ends_with(".htm") { "text/html".into() }
    else if path.ends_with(".css") { "text/css".into() }
    else if path.ends_with(".js") { "application/javascript".into() }
    else if path.ends_with(".json") { "application/json".into() }
    else if path.ends_with(".png") { "image/png".into() }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg".into() }
    else if path.ends_with(".svg") { "image/svg+xml".into() }
    else if path.ends_with(".ico") { "image/x-icon".into() }
    else if path.ends_with(".txt") { "text/plain".into() }
    else { "application/octet-stream".into() }
}

// ─── Filesystem ─────────────────────────────────────────

/// List files in a directory
pub fn list_dir(path: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        entries.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(entries)
}

/// Write content to a file
pub fn write_file(path: &str, content: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    Ok(std::fs::write(path, content)?)
}

/// Create directory (and parents)
pub fn create_dir(path: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    Ok(std::fs::create_dir_all(path)?)
}

/// Copy a single file
pub fn copy_file(from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    std::fs::copy(from, to)?;
    Ok(())
}

/// Copy a directory recursively
pub fn copy_dir(from: &str, to: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    fn copy_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dest = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_recursive(&entry.path(), &dest)?;
            } else {
                std::fs::copy(entry.path(), &dest)?;
            }
        }
        Ok(())
    }
    Ok(copy_recursive(std::path::Path::new(from), std::path::Path::new(to))?)
}

// ─── JSON ───────────────────────────────────────────────

/// Parse JSON string into serde_json::Value (panics on invalid JSON)
#[cfg(feature = "json")]
pub fn parse_json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("parse_json: {}", e))
}

/// Serialize serde_json::Value to JSON string
#[cfg(feature = "json")]
pub fn to_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Extension trait: .len() on serde_json::Value
#[cfg(feature = "json")]
pub trait JsonLen {
    fn len(&self) -> usize;
}

#[cfg(feature = "json")]
impl JsonLen for serde_json::Value {
    fn len(&self) -> usize {
        match self {
            serde_json::Value::Array(a) => a.len(),
            serde_json::Value::Object(o) => o.len(),
            serde_json::Value::String(s) => s.len(),
            _ => 0,
        }
    }
}

/// Get file stem (name without extension): "about.md" → "about"
pub fn path_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

// ─── String utilities ───────────────────────────────────

/// Check if string ends with suffix
pub fn ends_with(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

/// Check if string starts with prefix
pub fn starts_with(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string contains substring
pub fn contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

/// Replace all occurrences of `from` with `to`
pub fn replace(s: &str, from: &str, to: &str) -> String {
    s.replace(from, to)
}

/// Split string into lines
pub fn split_lines(s: &str) -> Vec<String> {
    s.lines().map(|l| l.to_string()).collect()
}

/// Split string by delimiter
pub fn split(s: &str, delim: &str) -> Vec<String> {
    s.split(delim).map(|p| p.to_string()).collect()
}

/// Find substring, return byte position or -1
pub fn find(s: &str, substr: &str) -> i64 {
    s.find(substr).map(|i| i as i64).unwrap_or(-1)
}

/// Find substring starting from byte position, return position or -1
pub fn find_from(s: &str, substr: &str, from: i64) -> i64 {
    let from = from.max(0) as usize;
    if from >= s.len() { return -1; }
    s[from..].find(substr).map(|i| (i + from) as i64).unwrap_or(-1)
}

/// Get substring from byte position to end (safe: adjusts to char boundaries)
pub fn slice_from(s: &str, from: i64) -> String {
    let mut from = from.max(0) as usize;
    if from >= s.len() { return String::new(); }
    while from < s.len() && !s.is_char_boundary(from) { from += 1; }
    s[from..].to_string()
}

/// Get substring from byte position to byte position (safe: adjusts to char boundaries)
pub fn slice_range(s: &str, from: i64, to: i64) -> String {
    let mut from = from.max(0) as usize;
    let mut to = to.max(0) as usize;
    if from >= s.len() || from >= to { return String::new(); }
    to = to.min(s.len());
    // Adjust to valid UTF-8 char boundaries
    while from < s.len() && !s.is_char_boundary(from) { from += 1; }
    while to > from && !s.is_char_boundary(to) { to -= 1; }
    if from >= to { return String::new(); }
    s[from..to].to_string()
}

/// Get string length in bytes
pub fn str_len(s: &str) -> i64 {
    s.len() as i64
}

/// Trim whitespace from both ends
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// Convert i64 to f64
pub fn float(n: i64) -> f64 {
    n as f64
}

/// Generate a range [0..n)
pub fn range(n: i64) -> Vec<i64> {
    (0..n).collect()
}

/// Generate a range [start..end)
pub fn range2(start: i64, end: i64) -> Vec<i64> {
    (start..end).collect()
}

/// Check if path is a directory
pub fn is_dir(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Catch panics — wraps std::panic::catch_unwind
pub fn catch<F: FnOnce()>(f: F) -> Result<(), String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            Err(msg)
        }
    }
}

/// Trigger a panic (catchable by catch())
pub fn error(msg: &str) {
    panic!("{}", msg);
}

// ─── HTTP (hyper) ─────────────────────────────────────────

#[cfg(feature = "http")]
pub mod http {
    use std::sync::Arc;
    use std::sync::Mutex as StdMutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::error::Error;
    use tokio::net::TcpListener;
    use tokio::sync::Notify;
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper::Request;
    use hyper::Response as HyperResponse;
    use hyper_util::rt::TokioIo;
    use http_body_util::Full;
    use bytes::Bytes;

    /// Unit struct — used as `HttpServer.listen(":3000")` in U source
    pub struct HttpServer;

    impl HttpServer {
        pub async fn listen(&self, addr: &str) -> Result<HttpListener, Box<dyn Error + Send + Sync>> {
            let bind_addr = if addr.starts_with(':') {
                format!("0.0.0.0{}", addr)
            } else {
                addr.to_string()
            };
            let listener = TcpListener::bind(&bind_addr).await?;
            Ok(HttpListener { listener })
        }
    }

    pub struct HttpListener {
        listener: TcpListener,
    }

    /// Zero-allocation bridge between hyper service and imperative U code.
    /// Uses Notify (permit-based) instead of channels — no heap alloc per request.
    struct Bridge {
        path: StdMutex<Option<String>>,
        response: StdMutex<Option<HttpResponse>>,
        req_ready: Notify,
        resp_ready: Notify,
        done: AtomicBool,
    }

    impl HttpListener {
        pub async fn accept(&self) -> Result<HttpConn, Box<dyn Error + Send + Sync>> {
            let (stream, _) = self.listener.accept().await?;
            stream.set_nodelay(true).ok();

            let bridge = Arc::new(Bridge {
                path: StdMutex::new(None),
                response: StdMutex::new(None),
                req_ready: Notify::new(),
                resp_ready: Notify::new(),
                done: AtomicBool::new(false),
            });

            let b_svc = bridge.clone();
            let b_done = bridge.clone();
            let io = TokioIo::new(stream);
            tokio::spawn(async move {
                let _ = hyper::server::conn::http1::Builder::new()
                    .keep_alive(true)
                    .pipeline_flush(true)
                    .serve_connection(io, service_fn(move |req: Request<Incoming>| {
                        let b = b_svc.clone();
                        async move {
                            let path = req.uri().path().to_string();
                            *b.path.lock().unwrap() = Some(path);
                            b.req_ready.notify_one();
                            b.resp_ready.notified().await;
                            let resp = b.response.lock().unwrap().take().unwrap_or_else(|| {
                                HttpResponse {
                                    status_code: 500,
                                    status_text: "Error".into(),
                                    content_type: "text/plain".into(),
                                    body: "Server Error".into(),
                                }
                            });
                            Ok::<_, hyper::Error>(HyperResponse::builder()
                                .status(resp.status_code)
                                .header("content-type", resp.content_type)
                                .body(Full::new(Bytes::from(resp.body)))
                                .unwrap())
                        }
                    }))
                    .await;
                b_done.done.store(true, Ordering::Release);
                b_done.req_ready.notify_one();
            });

            Ok(HttpConn { bridge })
        }
    }

    #[derive(Clone)]
    pub struct HttpConn {
        bridge: Arc<Bridge>,
    }

    impl HttpConn {
        /// Read next HTTP request path. Returns "" on connection close.
        pub async fn path(&self) -> String {
            self.bridge.req_ready.notified().await;
            if self.bridge.done.load(Ordering::Acquire) {
                return String::new();
            }
            self.bridge.path.lock().unwrap().take().unwrap_or_default()
        }

        pub async fn respond(&self, response: HttpResponse) {
            *self.bridge.response.lock().unwrap() = Some(response);
            self.bridge.resp_ready.notify_one();
        }
    }

    /// Unit struct — used as `Response.ok(...)` in U source
    pub struct Response;

    impl Response {
        pub fn ok(&self, body: impl Into<String>, content_type: impl Into<String>) -> HttpResponse {
            HttpResponse {
                status_code: 200,
                status_text: "OK".into(),
                content_type: content_type.into(),
                body: body.into(),
            }
        }

        pub fn not_found(&self, body: impl Into<String>) -> HttpResponse {
            HttpResponse {
                status_code: 404,
                status_text: "Not Found".into(),
                content_type: "text/plain".into(),
                body: body.into(),
            }
        }

        pub fn json(&self, body: impl Into<String>) -> HttpResponse {
            HttpResponse {
                status_code: 200,
                status_text: "OK".into(),
                content_type: "application/json".into(),
                body: body.into(),
            }
        }
    }

    pub struct HttpResponse {
        pub status_code: u16,
        pub status_text: String,
        pub content_type: String,
        pub body: String,
    }

    // ─── Request ──────────────────────────────────────────

    #[derive(Clone, Debug)]
    pub struct HttpRequest {
        pub method: String,
        pub path: String,
    }

    // ─── Router ───────────────────────────────────────────

    /// Unit struct — used as `Router.new()` in U source
    pub struct Router;

    impl Router {
        pub fn new(&self) -> URouter {
            URouter { routes: Vec::new(), fallback: None }
        }
    }

    /// Actual router: Vec of (method, path, handler). Linear scan — fast for small route tables.
    pub struct URouter {
        routes: Vec<(String, String, fn(HttpRequest) -> HttpResponse)>,
        fallback: Option<fn(HttpRequest) -> HttpResponse>,
    }

    impl URouter {
        pub fn get(&mut self, path: &str, handler: fn(HttpRequest) -> HttpResponse) {
            self.routes.push(("GET".into(), path.into(), handler));
        }

        pub fn post(&mut self, path: &str, handler: fn(HttpRequest) -> HttpResponse) {
            self.routes.push(("POST".into(), path.into(), handler));
        }

        pub fn fallback(&mut self, handler: fn(HttpRequest) -> HttpResponse) {
            self.fallback = Some(handler);
        }

        fn dispatch(&self, method: &str, path: &str, request: HttpRequest) -> HttpResponse {
            for (m, p, handler) in &self.routes {
                if m == method && p == path {
                    return handler(request);
                }
            }
            if let Some(fb) = self.fallback {
                fb(request)
            } else {
                HttpResponse {
                    status_code: 404, status_text: "Not Found".into(),
                    content_type: "text/plain".into(), body: "Not Found".into(),
                }
            }
        }
    }

    // ─── serve() — Router-based server, hyper direct ─────

    pub async fn serve(router: URouter, addr: &str) {
        let bind_addr = if addr.starts_with(':') {
            format!("0.0.0.0{}", addr)
        } else {
            addr.to_string()
        };
        let listener = TcpListener::bind(&bind_addr).await
            .unwrap_or_else(|e| panic!("Failed to bind {}: {}", bind_addr, e));
        let router = Arc::new(router);
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            stream.set_nodelay(true).ok();
            let router = router.clone();
            tokio::spawn(async move {
                let _ = hyper::server::conn::http1::Builder::new()
                    .keep_alive(true)
                    .pipeline_flush(true)
                    .serve_connection(TokioIo::new(stream), service_fn(move |req: Request<Incoming>| {
                        let router = router.clone();
                        async move {
                            let method = req.method().as_str().to_string();
                            let path = req.uri().path().to_string();
                            let request = HttpRequest { method: method.clone(), path: path.clone() };
                            let resp = router.dispatch(&method, &path, request);
                            Ok::<_, hyper::Error>(HyperResponse::builder()
                                .status(resp.status_code)
                                .header("content-type", resp.content_type)
                                .body(Full::new(Bytes::from(resp.body)))
                                .unwrap())
                        }
                    }))
                    .await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_sqlite_open_exec_query() {
        let db = Sqlite.open(":memory:").unwrap();
        db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
        db.exec1("INSERT INTO t (name) VALUES (?1)", &"hello").unwrap();
        let rows = db.query("SELECT id, name FROM t").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].int("id"), 1);
        assert_eq!(rows[0].string("name"), "hello");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_dollar_param_conversion() {
        let db = Sqlite.open(":memory:").unwrap();
        db.exec("CREATE TABLE t (v TEXT)").unwrap();
        db.exec1("INSERT INTO t (v) VALUES ($1)", &"test").unwrap();
        let rows = db.query("SELECT v FROM t").unwrap();
        assert_eq!(rows[0].string("v"), "test");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_exec1_int() {
        let db = Sqlite.open(":memory:").unwrap();
        db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)").unwrap();
        db.exec1("INSERT INTO t (v) VALUES ($1)", &42_i64).unwrap();
        let rows = db.query("SELECT v FROM t").unwrap();
        assert_eq!(rows[0].int("v"), 42);
    }

    #[tokio::test]
    async fn test_channel() {
        let ch = Channel.new();
        let ch2 = ch.clone();
        tokio::spawn(async move { ch2.send("hello"); });
        let msg = ch.recv().await;
        assert_eq!(msg, "hello");
    }

    #[test]
    fn test_str_ext_int() {
        assert_eq!("42".int().unwrap(), 42);
        assert_eq!(String::from("7").int().unwrap(), 7);
        assert!("abc".int().is_err());
    }

    #[test]
    fn test_string_utils() {
        assert!(starts_with("hello", "hel"));
        assert!(!starts_with("hello", "xyz"));
        assert!(ends_with("hello.md", ".md"));
        assert!(contains("hello world", "world"));
        assert_eq!(replace("hello world", "world", "U"), "hello U");
        assert_eq!(find("hello", "ll"), 2);
        assert_eq!(find("hello", "xyz"), -1);
        assert_eq!(find_from("abcabc", "abc", 1), 3);
        assert_eq!(slice_from("hello", 2), "llo");
        assert_eq!(slice_range("hello", 1, 4), "ell");
        assert_eq!(str_len("hello"), 5);
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(path_stem("about.md"), "about");
    }

    #[test]
    fn test_split_lines() {
        let lines = split_lines("a\nb\nc");
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_file_ops() {
        let dir = std::env::temp_dir().join("u_test_fileops");
        create_dir(dir.to_str().unwrap()).unwrap();
        let fpath = format!("{}/test.txt", dir.display());
        write_file(&fpath, "hello").unwrap();
        let files = list_dir(dir.to_str().unwrap()).unwrap();
        assert!(files.contains(&"test.txt".to_string()));
        let content = read_file(&fpath).unwrap();
        assert_eq!(content, "hello");
        std::fs::remove_dir_all(&dir).ok();
    }
}

// Int Channel для U-lang
pub mod int_channel {
    use std::sync::mpsc::{channel, Sender, Receiver};
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Unit struct — used as `IntChannel.new()` in U source
    pub struct IntChannel;

    impl IntChannel {
        pub fn new(&self) -> IntChan {
            let (tx, rx) = channel();
            IntChan { sender: tx, receiver: Arc::new(Mutex::new(rx)) }
        }
    }

    /// The actual int channel value, cloneable across threads
    #[derive(Clone)]
    pub struct IntChan {
        sender: Sender<i64>,
        receiver: Arc<Mutex<Receiver<i64>>>,
    }

    impl IntChan {
        pub fn send(&self, val: i64) {
            let _ = self.sender.send(val);
        }

        pub fn recv(&self) -> i64 {
            self.receiver.lock().unwrap().recv().unwrap_or(0)
        }
    }
}

// Async Int Channel для U-lang с tokio
pub mod async_int_channel {
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub struct AsyncIntChannel;

    impl AsyncIntChannel {
        pub fn new(&self) -> AsyncIntChan {
            let (tx, rx) = unbounded_channel::<i64>();
            AsyncIntChan { sender: tx, receiver: Arc::new(Mutex::new(rx)) }
        }
    }

    #[derive(Clone)]
    pub struct AsyncIntChan {
        sender: UnboundedSender<i64>,
        receiver: Arc<Mutex<UnboundedReceiver<i64>>>,
    }

    impl AsyncIntChan {
        pub fn send(&self, val: i64) {
            let _ = self.sender.send(val);
        }

        pub async fn recv(&self) -> i64 {
            self.receiver.lock().await.recv().await.unwrap_or(0)
        }

        pub fn try_recv(&self) -> Option<i64> {
            // Non-blocking receive - would need a different approach for mpsc
            // For now, return None (channel empty or closed)
            None
        }
    }
}

// Async String Channel для U-lang с tokio
pub mod async_string_channel {
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub struct AsyncStringChannel;

    impl AsyncStringChannel {
        pub fn new(&self) -> AsyncStringChan {
            let (tx, rx) = unbounded_channel::<String>();
            AsyncStringChan { sender: tx, receiver: Arc::new(Mutex::new(rx)) }
        }
    }

    #[derive(Clone)]
    pub struct AsyncStringChan {
        sender: UnboundedSender<String>,
        receiver: Arc<Mutex<UnboundedReceiver<String>>>,
    }

    impl AsyncStringChan {
        pub fn send(&self, val: &str) {
            let _ = self.sender.send(val.to_string());
        }

        pub async fn recv(&self) -> String {
            self.receiver.lock().await.recv().await.unwrap_or_default()
        }

        pub fn try_recv(&self) -> Option<String> {
            None
        }
    }
}

// Async Float Channel для U-lang с tokio
pub mod async_float_channel {
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub struct AsyncFloatChannel;

    impl AsyncFloatChannel {
        pub fn new(&self) -> AsyncFloatChan {
            let (tx, rx) = unbounded_channel::<f64>();
            AsyncFloatChan { sender: tx, receiver: Arc::new(Mutex::new(rx)) }
        }
    }

    #[derive(Clone)]
    pub struct AsyncFloatChan {
        sender: UnboundedSender<f64>,
        receiver: Arc<Mutex<UnboundedReceiver<f64>>>,
    }

    impl AsyncFloatChan {
        pub fn send(&self, val: f64) {
            let _ = self.sender.send(val);
        }

        pub async fn recv(&self) -> f64 {
            self.receiver.lock().await.recv().await.unwrap_or(0.0)
        }

        pub fn try_recv(&self) -> Option<f64> {
            None
        }
    }
}

// Async Bool Channel для U-lang с tokio
pub mod async_bool_channel {
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub struct AsyncBoolChannel;

    impl AsyncBoolChannel {
        pub fn new(&self) -> AsyncBoolChan {
            let (tx, rx) = unbounded_channel::<bool>();
            AsyncBoolChan { sender: tx, receiver: Arc::new(Mutex::new(rx)) }
        }
    }

    #[derive(Clone)]
    pub struct AsyncBoolChan {
        sender: UnboundedSender<bool>,
        receiver: Arc<Mutex<UnboundedReceiver<bool>>>,
    }

    impl AsyncBoolChan {
        pub fn send(&self, val: bool) {
            let _ = self.sender.send(val);
        }

        pub async fn recv(&self) -> bool {
            self.receiver.lock().await.recv().await.unwrap_or(false)
        }

        pub fn try_recv(&self) -> Option<bool> {
            None
        }
    }
}
