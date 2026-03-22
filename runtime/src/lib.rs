//! U Language Runtime
//!
//! Provides Sqlite, Args, concurrency (tokio), HTTP, and string extensions for U scripts.

use std::collections::HashMap;
use std::error::Error;

// Re-export everything at crate root for `use u_runtime::*;`
pub use db::{Db, Row, Sqlite};
pub use args::{Args, ParsedArgs};
pub use concurrency::{Channel, Chan};
pub use http::{HttpServer, HttpListener, HttpConn, Response, HttpResponse};

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

// ─── Sqlite ──────────────────────────────────────────────

pub mod db {
    use super::*;
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

/// Get substring from byte position to end
pub fn slice_from(s: &str, from: i64) -> String {
    let from = from.max(0) as usize;
    if from >= s.len() { return String::new(); }
    s[from..].to_string()
}

/// Get substring from byte position to byte position
pub fn slice_range(s: &str, from: i64, to: i64) -> String {
    let from = from.max(0) as usize;
    let to = to.max(0) as usize;
    if from >= s.len() || from >= to { return String::new(); }
    let to = to.min(s.len());
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

// ─── HTTP (tokio) ────────────────────────────────────────

pub mod http {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::error::Error;

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

    impl HttpListener {
        pub async fn accept(&self) -> Result<HttpConn, Box<dyn Error + Send + Sync>> {
            let (stream, _) = self.listener.accept().await?;
            stream.set_nodelay(true).ok();
            Ok(HttpConn {
                stream: Arc::new(Mutex::new(stream)),
                keep_alive: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            })
        }
    }

    fn parse_path(request: &str) -> String {
        if let Some(line) = request.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].to_string();
            }
        }
        String::new()
    }

    #[derive(Clone)]
    pub struct HttpConn {
        stream: Arc<Mutex<TcpStream>>,
        keep_alive: Arc<std::sync::atomic::AtomicBool>,
    }

    impl HttpConn {
        /// Read next HTTP request from connection. Returns path, or "" on EOF/closed.
        pub async fn path(&self) -> String {
            // If previous request was not keep-alive, stop reading
            if !self.keep_alive.load(std::sync::atomic::Ordering::Relaxed) {
                return String::new();
            }
            let mut stream = self.stream.lock().await;
            let mut buf = [0u8; 4096];
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => String::new(),
                Ok(n) => {
                    let request = String::from_utf8_lossy(&buf[..n]);
                    // Check if client wants keep-alive
                    let ka = request.to_ascii_lowercase().contains("keep-alive");
                    self.keep_alive.store(ka, std::sync::atomic::Ordering::Relaxed);
                    parse_path(&request)
                }
            }
        }

        pub async fn respond(&self, response: HttpResponse) {
            let ka = self.keep_alive.load(std::sync::atomic::Ordering::Relaxed);
            let conn_header = if ka { "keep-alive" } else { "close" };
            let mut stream = self.stream.lock().await;
            let resp = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n{}",
                response.status_code, response.status_text,
                response.content_type, response.body.len(), conn_header, response.body
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.flush().await;
            // If not keep-alive, shut down write side so server sends FIN first
            if !ka {
                let _ = stream.shutdown().await;
            }
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
    }

    pub struct HttpResponse {
        pub status_code: u16,
        pub status_text: String,
        pub content_type: String,
        pub body: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_dollar_param_conversion() {
        let db = Sqlite.open(":memory:").unwrap();
        db.exec("CREATE TABLE t (v TEXT)").unwrap();
        db.exec1("INSERT INTO t (v) VALUES ($1)", &"test").unwrap();
        let rows = db.query("SELECT v FROM t").unwrap();
        assert_eq!(rows[0].string("v"), "test");
    }

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
