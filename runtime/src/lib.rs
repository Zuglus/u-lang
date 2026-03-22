//! U Language Runtime
//!
//! Provides Sqlite, Args, and string extensions for U scripts.

use std::collections::HashMap;
use std::error::Error;

// Re-export everything at crate root for `use u_runtime::*;`
pub use db::{Db, Row, Sqlite};
pub use args::{Args, ParsedArgs};

/// Extension trait: .int() on strings
pub trait StrExt {
    fn int(&self) -> Result<i64, Box<dyn Error>>;
}

impl StrExt for String {
    fn int(&self) -> Result<i64, Box<dyn Error>> {
        Ok(self.parse::<i64>()?)
    }
}

impl StrExt for str {
    fn int(&self) -> Result<i64, Box<dyn Error>> {
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
        pub fn open(&self, path: &str) -> Result<Db, Box<dyn Error>> {
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
        pub fn exec(&self, sql: &str) -> Result<(), Box<dyn Error>> {
            let sql = convert_params(sql);
            self.conn.execute(&sql, ())?;
            Ok(())
        }

        /// Execute with one param (any type implementing ToSql)
        pub fn exec1<T: rusqlite::types::ToSql>(&self, sql: &str, param: &T) -> Result<(), Box<dyn Error>> {
            let sql = convert_params(sql);
            self.conn.execute(&sql, [param as &dyn rusqlite::types::ToSql])?;
            Ok(())
        }

        /// Query without params
        pub fn query(&self, sql: &str) -> Result<Vec<Row>, Box<dyn Error>> {
            self.query_internal(&convert_params(sql), ())
        }

        /// Query with one param
        pub fn query1<T: rusqlite::types::ToSql>(&self, sql: &str, param: &T) -> Result<Vec<Row>, Box<dyn Error>> {
            self.query_internal(&convert_params(sql), [param as &dyn rusqlite::types::ToSql])
        }

        fn query_internal<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<Vec<Row>, Box<dyn Error>> {
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
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
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
        /// Get a required argument. Joins all positional args (for multi-word values).
        pub fn require(&self, _name: &str) -> Result<String, Box<dyn Error>> {
            if self.positional.is_empty() {
                return Err(format!("Отсутствует аргумент: <{}>", _name).into());
            }
            Ok(self.positional.join(" "))
        }
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

    #[test]
    fn test_str_ext_int() {
        assert_eq!("42".int().unwrap(), 42);
        assert_eq!(String::from("7").int().unwrap(), 7);
        assert!("abc".int().is_err());
    }
}
