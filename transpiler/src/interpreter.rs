use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::panic::AssertUnwindSafe;
use crate::ast::*;
use std::fmt;

// ── Value ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ChannelPair {
    tx: mpsc::Sender<Value>,
    rx: Arc<Mutex<mpsc::Receiver<Value>>>,
}

impl fmt::Debug for ChannelPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel")
    }
}

#[derive(Clone)]
struct DbHandle(Arc<Mutex<rusqlite::Connection>>);

impl fmt::Debug for DbHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Db")
    }
}

#[derive(Debug, Clone)]
#[allow(private_interfaces)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    Struct { type_name: String, fields: HashMap<String, Value> },
    Variant { name: String, values: Vec<Value> },
    Channel(ChannelPair),
    Lambda { params: Vec<String>, body: Expr },
    Db(DbHandle),
    Type(String),
    None,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 { write!(f, "{}", *n as i64) }
                else { write!(f, "{}", n) }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Struct { type_name, fields } => {
                write!(f, "{}(", type_name)?;
                let mut first = true;
                for (k, v) in fields {
                    if !first { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                    first = false;
                }
                write!(f, ")")
            }
            Value::Variant { name, values } => {
                write!(f, "{}(", name)?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Channel(_) => write!(f, "<Channel>"),
            Value::Lambda { .. } => write!(f, "<fn>"),
            Value::Db(_) => write!(f, "<Db>"),
            Value::Type(name) => write!(f, "<type {}>", name),
            Value::None => write!(f, "none"),
        }
    }
}

// ── Internal types ─────────────────────────────────────────────────────

#[derive(Clone)]
struct FnDef {
    params: Vec<String>,
    has_mut_params: bool,
    is_test: bool,
    body: Vec<Stmt>,
}

enum Signal {
    None,
    Return(Value),
}

// ── Interpreter ────────────────────────────────────────────────────────

pub struct Interpreter {
    scopes: Vec<HashMap<String, Value>>,
    fns: HashMap<String, FnDef>,
    methods: HashMap<String, HashMap<String, FnDef>>,
    variant_fields: HashMap<String, Vec<String>>,
    cli_args: Vec<String>,
}

pub fn has_memory_decl(program: &Program) -> bool {
    program.statements.iter().any(|s| matches!(s, Stmt::MemoryDecl { .. }))
}

fn val_str(args: &[Value], i: usize, ctx: &str) -> Result<String, String> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(v) => Ok(v.to_string()),
        None => Err(format!("{}: missing arg {}", ctx, i)),
    }
}

fn val_int(args: &[Value], i: usize, ctx: &str) -> Result<i64, String> {
    match args.get(i) {
        Some(Value::Int(n)) => Ok(*n),
        _ => Err(format!("{}: expected int arg {}", ctx, i)),
    }
}

fn sql_params(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            out.push('?');
        } else {
            out.push(bytes[i] as char);
        }
        i += 1;
    }
    out
}

fn to_sql(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Int(n) => rusqlite::types::Value::Integer(*n),
        Value::Float(f) => rusqlite::types::Value::Real(*f),
        Value::Str(s) => rusqlite::types::Value::Text(s.clone()),
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        _ => rusqlite::types::Value::Null,
    }
}

fn from_sql(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Integer(n) => Value::Int(n),
        rusqlite::types::Value::Real(f) => Value::Float(f),
        rusqlite::types::Value::Text(s) => Value::Str(s),
        rusqlite::types::Value::Null => Value::None,
        rusqlite::types::Value::Blob(_) => Value::None,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::None, Value::None) => true,
        _ => false,
    }
}

fn ok_val(v: Value) -> Value {
    Value::Variant { name: "Ok".into(), values: vec![v] }
}

fn err_val(msg: String) -> Value {
    Value::Variant { name: "Err".into(), values: vec![Value::Str(msg)] }
}

impl Interpreter {
    pub fn new(cli_args: Vec<String>) -> Self {
        let mut scopes = vec![HashMap::new()];
        scopes[0].insert("none".into(), Value::None);
        scopes[0].insert("Channel".into(), Value::Type("Channel".into()));
        scopes[0].insert("Sqlite".into(), Value::Type("Sqlite".into()));
        scopes[0].insert("Args".into(), Value::Type("Args".into()));
        Interpreter { scopes, fns: HashMap::new(), methods: HashMap::new(), variant_fields: HashMap::new(), cli_args }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), String> {
        // Suppress default panic output — spawn uses catch_unwind with its own reporting
        std::panic::set_hook(Box::new(|_| {}));
        for stmt in &program.statements {
            if let Signal::Return(_) = self.exec_stmt(stmt)? {
                break;
            }
        }
        Ok(())
    }

    pub fn run_tests(&mut self, program: &Program) -> (usize, usize) {
        std::panic::set_hook(Box::new(|_| {}));
        // Phase 1: register definitions only (skip executable top-level code)
        for stmt in &program.statements {
            match stmt {
                Stmt::FnDef { .. } | Stmt::StructDef { .. } | Stmt::ImplBlock { .. }
                | Stmt::TraitDef { .. } | Stmt::TypeDef { .. } | Stmt::UseDecl { .. }
                | Stmt::MemoryDecl { .. } => { let _ = self.exec_stmt(stmt); }
                _ => {}
            }
        }
        // Phase 2: collect and run test functions
        let test_names: Vec<String> = self.fns.iter()
            .filter(|(_, d)| d.is_test)
            .map(|(n, _)| n.clone())
            .collect();
        let mut passed = 0;
        let mut failed = 0;
        for name in &test_names {
            print!("test {} ... ", name);
            match self.call_fn(name, vec![]) {
                Ok(_) => { println!("ok"); passed += 1; }
                Err(e) => { println!("FAILED"); eprintln!("  {}", e); failed += 1; }
            }
        }
        (passed, failed)
    }

    // ── Scope helpers ──────────────────────────────────────────────────

    fn set_var(&mut self, name: &str, val: Value) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), val);
                return;
            }
        }
        self.scopes.last_mut().unwrap().insert(name.to_string(), val);
    }

    fn get_var(&self, name: &str) -> Result<Value, String> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Ok(v.clone());
            }
        }
        match name {
            "TcpListener" | "HttpServer" =>
                Err(format!("{} requires compiled mode. Use: u build <file>", name)),
            _ => Err(format!("undefined variable: {}", name)),
        }
    }

    // ── Statement execution ────────────────────────────────────────────

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Signal, String> {
        match stmt {
            Stmt::Assignment { name, value, .. } => {
                let val = self.eval_expr(value)?;
                self.set_var(name, val);
                Ok(Signal::None)
            }
            Stmt::ExprStmt { expr, .. } => {
                self.eval_expr(expr)?;
                Ok(Signal::None)
            }
            Stmt::FnDef { name, params, body, is_test, .. } => {
                self.fns.insert(name.clone(), FnDef {
                    params: params.iter().map(|p| p.name.clone()).collect(),
                    has_mut_params: params.iter().any(|p| p.is_mut),
                    is_test: *is_test,
                    body: body.clone(),
                });
                Ok(Signal::None)
            }
            Stmt::Return { value, .. } => {
                let val = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::None,
                };
                Ok(Signal::Return(val))
            }
            Stmt::If { condition, body, elifs, else_body, .. } => {
                let cv = self.eval_expr(condition)?;
                if self.is_truthy(&cv) {
                    return self.exec_block(body);
                }
                for (cond, block) in elifs {
                    let cv = self.eval_expr(cond)?;
                    if self.is_truthy(&cv) {
                        return self.exec_block(block);
                    }
                }
                if let Some(block) = else_body {
                    return self.exec_block(block);
                }
                Ok(Signal::None)
            }
            Stmt::ForLoop { pattern, iter, body, .. } => {
                let items = match self.eval_expr(iter)? {
                    Value::List(items) => items,
                    _ => return Err("for: expected list".into()),
                };
                for item in items {
                    match pattern {
                        ForPattern::Single(name) => self.set_var(name, item),
                        ForPattern::Tuple(names) => {
                            if let Value::List(tuple) = item {
                                for (i, n) in names.iter().enumerate() {
                                    self.set_var(n, tuple.get(i).cloned().unwrap_or(Value::None));
                                }
                            }
                        }
                    }
                    if let Signal::Return(v) = self.exec_block(body)? {
                        return Ok(Signal::Return(v));
                    }
                }
                Ok(Signal::None)
            }
            Stmt::Loop { body, .. } => {
                loop {
                    if let Signal::Return(v) = self.exec_block(body)? {
                        return Ok(Signal::Return(v));
                    }
                }
            }
            Stmt::Match { expr, arms, .. } => {
                let val = self.eval_expr(expr)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val) {
                        for (name, v) in bindings {
                            self.set_var(&name, v);
                        }
                        return self.exec_stmt(&arm.body);
                    }
                }
                Ok(Signal::None)
            }
            Stmt::MutAssign { object, field, value, .. } => {
                let val = self.eval_expr(value)?;
                if let Expr::Identifier { name, .. } = object {
                    let mut obj = self.get_var(name)?;
                    if let Value::Struct { ref mut fields, .. } = obj {
                        fields.insert(field.clone(), val);
                    }
                    self.set_var(name, obj);
                }
                Ok(Signal::None)
            }
            Stmt::StructDef { name, .. } => {
                self.set_var(name, Value::Type(name.clone()));
                Ok(Signal::None)
            }
            Stmt::ImplBlock { target, methods: method_stmts, .. } => {
                for m in method_stmts {
                    if let Stmt::FnDef { name, params, body, .. } = m {
                        self.methods
                            .entry(target.clone())
                            .or_default()
                            .insert(name.clone(), FnDef {
                                params: params.iter().map(|p| p.name.clone()).collect(),
                                has_mut_params: params.iter().any(|p| p.is_mut),
                                is_test: false,
                                body: body.clone(),
                            });
                    }
                }
                Ok(Signal::None)
            }
            Stmt::Spawn { expr, .. } => self.exec_spawn(expr),
            Stmt::TypeDef { variants, .. } => {
                for v in variants {
                    self.variant_fields.insert(
                        v.name.clone(),
                        v.fields.iter().map(|f| f.name.clone()).collect(),
                    );
                }
                Ok(Signal::None)
            }
            Stmt::UseDecl { path, .. } => {
                let compiled_only = ["std.http", "std.tcp"];
                if let Some(mod_name) = compiled_only.iter().find(|p| path.contains(*p)) {
                    return Err(format!(
                        "{} requires compiled mode. Use: u build <file>", mod_name));
                }
                Ok(Signal::None)
            }
            // Skip: trait signatures not needed at runtime
            Stmt::TraitDef { .. } | Stmt::MemoryDecl { .. } => {
                Ok(Signal::None)
            }
        }
    }

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Signal, String> {
        for stmt in stmts {
            if let Signal::Return(v) = self.exec_stmt(stmt)? {
                return Ok(Signal::Return(v));
            }
        }
        Ok(Signal::None)
    }

    fn match_pattern(&self, pattern: &MatchPattern, val: &Value) -> Option<Vec<(String, Value)>> {
        match (pattern, val) {
            (MatchPattern::Wildcard, _) => Some(vec![]),
            (MatchPattern::StringLit(s), Value::Str(v)) if s == v => Some(vec![]),
            (MatchPattern::Variant { name, bindings }, Value::Variant { name: vn, values })
                if name == vn =>
            {
                Some(bindings.iter().zip(values.iter())
                    .map(|(b, v)| (b.clone(), v.clone()))
                    .collect())
            }
            // Enum variants stored as Struct — bind fields positionally
            (MatchPattern::Variant { name, bindings }, Value::Struct { type_name, fields })
                if name == type_name =>
            {
                if let Some(field_order) = self.variant_fields.get(name) {
                    Some(bindings.iter().zip(field_order.iter())
                        .map(|(b, f)| (b.clone(), fields.get(f).cloned().unwrap_or(Value::None)))
                        .collect())
                } else {
                    // No TypeDef — fallback: bind fields in iteration order
                    Some(bindings.iter().zip(fields.values())
                        .map(|(b, v)| (b.clone(), v.clone()))
                        .collect())
                }
            }
            _ => None,
        }
    }

    // ── Spawn ──────────────────────────────────────────────────────────

    fn exec_spawn(&mut self, expr: &Expr) -> Result<Signal, String> {
        let (name, args) = match expr {
            Expr::FunctionCall { name, args, .. } => (name, args),
            _ => return Err("spawn: expected function call".into()),
        };

        let arg_vals: Vec<_> = args.iter()
            .map(|a| self.eval_expr(a))
            .collect::<Result<_, _>>()?;

        let def = self.fns.get(name.as_str()).cloned()
            .ok_or_else(|| format!("spawn: undefined function: {}", name))?;

        if def.has_mut_params {
            return Err(format!(
                "spawn: function '{}' has :: params — cannot spawn with mutable references", name));
        }

        if arg_vals.len() != def.params.len() {
            return Err(format!("spawn {}: expected {} args, got {}",
                name, def.params.len(), arg_vals.len()));
        }

        let fns = self.fns.clone();
        let methods = self.methods.clone();
        let variant_fields = self.variant_fields.clone();
        let cli_args = self.cli_args.clone();

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut interp = Interpreter {
                    scopes: vec![HashMap::new()],
                    fns,
                    methods,
                    variant_fields,
                    cli_args,
                };
                interp.scopes[0].insert("Channel".into(), Value::Type("Channel".into()));
                interp.scopes[0].insert("Sqlite".into(), Value::Type("Sqlite".into()));
                interp.scopes[0].insert("Args".into(), Value::Type("Args".into()));
                for (param, val) in def.params.iter().zip(arg_vals) {
                    interp.scopes[0].insert(param.clone(), val);
                }
                interp.exec_block(&def.body)
            }));

            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => eprintln!("spawn error: {}", e),
                Err(panic_val) => {
                    let msg = panic_val
                        .downcast::<String>().map(|s| *s)
                        .or_else(|e| e.downcast::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|_| "unknown panic".into());
                    eprintln!("spawn panicked: {}", msg);
                }
            }
        });

        Ok(Signal::None)
    }

    // ── Expression evaluation ──────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::IntLiteral { value, .. } => Ok(Value::Int(*value)),
            Expr::FloatLiteral { value, .. } => Ok(Value::Float(*value)),
            Expr::BoolLiteral { value, .. } => Ok(Value::Bool(*value)),
            Expr::StringLiteral { parts, .. } => {
                let mut s = String::new();
                for part in parts {
                    match part {
                        StringPart::Text(t) => s.push_str(t),
                        StringPart::Interpolation(e) => s.push_str(&self.eval_expr(e)?.to_string()),
                    }
                }
                Ok(Value::Str(s))
            }
            Expr::Identifier { name, .. } => self.get_var(name),
            Expr::List { elements, .. } => {
                let vals: Result<Vec<_>, _> = elements.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::List(vals?))
            }
            Expr::FunctionCall { name, args, .. } => {
                let av: Vec<_> = args.iter().map(|a| self.eval_expr(a)).collect::<Result<_, _>>()?;
                self.call_fn(name, av)
            }
            Expr::BinaryOp { left, op, right, .. } => {
                let lv = self.eval_expr(left)?;
                let rv = self.eval_expr(right)?;
                self.eval_binop(&lv, op, &rv)
            }
            Expr::UnaryOp { op, expr, .. } => {
                let val = self.eval_expr(expr)?;
                match op.as_str() {
                    "-" => match val {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err("unary -: expected number".into()),
                    },
                    "not" => Ok(Value::Bool(!self.is_truthy(&val))),
                    _ => Err(format!("unknown unary op: {}", op)),
                }
            }
            Expr::MethodCall { object, method, args, .. } => {
                self.eval_method_call(object, method, args)
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj = self.eval_expr(object)?;
                match obj {
                    Value::Struct { fields, .. } => {
                        fields.get(field).cloned()
                            .ok_or_else(|| format!("no field .{}", field))
                    }
                    _ => Err(format!("field access .{} on non-struct", field)),
                }
            }
            Expr::StructInit { name, fields, .. } => {
                let mut fv = HashMap::new();
                for (fname, fexpr) in fields {
                    fv.insert(fname.clone(), self.eval_expr(fexpr)?);
                }
                Ok(Value::Struct { type_name: name.clone(), fields: fv })
            }
            Expr::PostfixOp { expr, op, .. } => {
                let val = self.eval_expr(expr)?;
                match op.as_str() {
                    "?" => match val {
                        Value::Variant { name, mut values } if name == "Ok" =>
                            Ok(values.pop().unwrap_or(Value::None)),
                        Value::Variant { name, values } if name == "Err" =>
                            Err(values.first().map(|v| v.to_string()).unwrap_or_default()),
                        other => Ok(other),
                    },
                    "!" => match val {
                        Value::Variant { name, mut values } if name == "Ok" =>
                            Ok(values.pop().unwrap_or(Value::None)),
                        Value::Variant { name, values } if name == "Err" => {
                            let msg = values.first().map(|v| v.to_string()).unwrap_or_default();
                            panic!("unwrap on Err: {}", msg);
                        }
                        other => Ok(other),
                    },
                    _ => Ok(val),
                }
            }
            Expr::Lambda { params, body, .. } => {
                Ok(Value::Lambda { params: params.clone(), body: *body.clone() })
            }
        }
    }

    // ── Method calls (channel + struct + built-in) ─────────────────────

    fn eval_method_call(&mut self, object: &Expr, method: &str, args: &[Expr]) -> Result<Value, String> {
        let obj = self.eval_expr(object)?;

        // Channel methods — before arg evaluation for recv (no args)
        if let Value::Channel(ref ch) = obj {
            match method {
                "send" => {
                    let val = if let Some(a) = args.first() {
                        self.eval_expr(a)?
                    } else { Value::None };
                    ch.tx.send(val).map_err(|_| "channel closed".to_string())?;
                    return Ok(Value::None);
                }
                "recv" => {
                    let rx = ch.rx.lock().map_err(|_| "channel lock poisoned".to_string())?;
                    return rx.recv().map_err(|_| "channel closed".to_string());
                }
                _ => {}
            }
        }

        // Db methods
        if let Value::Db(ref handle) = obj {
            let arg_vals: Vec<_> = args.iter().map(|a| self.eval_expr(a)).collect::<Result<_, _>>()?;
            return self.db_method(handle, method, arg_vals);
        }

        let arg_vals: Vec<_> = args.iter().map(|a| self.eval_expr(a)).collect::<Result<_, _>>()?;

        // Resolve type name for struct/type method lookup
        let type_name = match &obj {
            Value::Struct { type_name, .. } => Some(type_name.clone()),
            Value::Type(n) => Some(n.clone()),
            _ => None,
        };

        if let Some(ref tn) = type_name {
            // Channel.new()
            if tn == "Channel" && method == "new" {
                let (tx, rx) = mpsc::channel();
                return Ok(Value::Channel(ChannelPair {
                    tx,
                    rx: Arc::new(Mutex::new(rx)),
                }));
            }

            // Sqlite.open(path)
            if tn == "Sqlite" && method == "open" {
                let path = val_str(&arg_vals, 0, "Sqlite.open")?;
                return match rusqlite::Connection::open(&path) {
                    Ok(conn) => Ok(ok_val(Value::Db(DbHandle(Arc::new(Mutex::new(conn)))))),
                    Err(e) => Ok(err_val(e.to_string())),
                };
            }

            // Args.parse()
            if tn == "Args" && method == "parse" {
                let command = self.cli_args.first().cloned().unwrap_or_default();
                let positional: Vec<Value> = self.cli_args.iter().skip(1)
                    .map(|s| Value::Str(s.clone())).collect();
                return Ok(Value::Struct {
                    type_name: "Args".into(),
                    fields: HashMap::from([
                        ("command".into(), Value::Str(command)),
                        ("_positional".into(), Value::List(positional)),
                    ]),
                });
            }

            // User-defined methods (impl blocks)
            if let Some(def) = self.methods.get(tn).and_then(|m| m.get(method)).cloned() {
                let has_self = def.params.first().map(|p| p == "self").unwrap_or(false);
                let offset = if has_self { 1 } else { 0 };

                let global = self.scopes[0].clone();
                let saved = std::mem::replace(&mut self.scopes, vec![global, HashMap::new()]);
                if has_self {
                    self.scopes[1].insert("self".into(), obj.clone());
                }
                for (i, val) in arg_vals.into_iter().enumerate() {
                    if let Some(pname) = def.params.get(i + offset) {
                        self.scopes[1].insert(pname.clone(), val);
                    }
                }

                let result = match self.exec_block(&def.body)? {
                    Signal::Return(v) => v,
                    Signal::None => Value::None,
                };

                let modified_self = self.scopes[1].get("self").cloned();
                self.scopes = saved;

                if let Some(ms) = modified_self {
                    if let Expr::Identifier { name, .. } = object {
                        self.set_var(name, ms);
                    }
                }

                return Ok(result);
            }
        }

        // Built-in methods on values
        self.builtin_method(obj, method, arg_vals)
    }

    fn db_method(&self, handle: &DbHandle, method: &str, args: Vec<Value>) -> Result<Value, String> {
        let conn = handle.0.lock().map_err(|_| "db lock poisoned".to_string())?;
        match method {
            "exec" => {
                let sql = sql_params(&val_str(&args, 0, "db.exec")?);
                let params: Vec<rusqlite::types::Value> = args[1..].iter().map(to_sql).collect();
                let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql).collect();
                match conn.execute(&sql, refs.as_slice()) {
                    Ok(_) => Ok(ok_val(Value::None)),
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            "query" => {
                let sql = sql_params(&val_str(&args, 0, "db.query")?);
                let params: Vec<rusqlite::types::Value> = args[1..].iter().map(to_sql).collect();
                let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql).collect();
                let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                let rows = stmt.query_map(refs.as_slice(), |row| {
                    let mut fields = HashMap::new();
                    for (i, name) in col_names.iter().enumerate() {
                        let val: rusqlite::types::Value = row.get(i)?;
                        fields.insert(name.clone(), from_sql(val));
                    }
                    Ok(Value::Struct { type_name: "Row".into(), fields })
                }).map_err(|e| e.to_string())?;
                let result: Result<Vec<_>, _> = rows.map(|r| r.map_err(|e| e.to_string())).collect();
                Ok(ok_val(Value::List(result?)))
            }
            "query_one" => {
                let sql = sql_params(&val_str(&args, 0, "db.query_one")?);
                let params: Vec<rusqlite::types::Value> = args[1..].iter().map(to_sql).collect();
                let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter()
                    .map(|p| p as &dyn rusqlite::types::ToSql).collect();
                let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
                let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
                let mut rows = stmt.query(refs.as_slice()).map_err(|e| e.to_string())?;
                match rows.next().map_err(|e| e.to_string())? {
                    Some(row) => {
                        let mut fields = HashMap::new();
                        for (i, name) in col_names.iter().enumerate() {
                            let val: rusqlite::types::Value = row.get(i).map_err(|e| e.to_string())?;
                            fields.insert(name.clone(), from_sql(val));
                        }
                        Ok(ok_val(Value::Struct { type_name: "Row".into(), fields }))
                    }
                    None => Ok(ok_val(Value::None)),
                }
            }
            "last_insert_id" => Ok(Value::Int(conn.last_insert_rowid())),
            _ => Err(format!("unknown Db method: {}", method)),
        }
    }

    fn list_lambda_method(&mut self, obj: Value, method: &str, args: Vec<Value>) -> Result<Value, String> {
        let list = match obj {
            Value::List(l) => l,
            _ => return Err(format!(".{} requires a list", method)),
        };
        let (params, body) = match args.into_iter().next() {
            Some(Value::Lambda { params, body }) => (params, body),
            _ => return Err(format!(".{} requires a lambda argument", method)),
        };
        match method {
            "filter" => {
                let mut result = Vec::new();
                for item in &list {
                    self.scopes.push(HashMap::new());
                    if let Some(p) = params.first() {
                        self.scopes.last_mut().unwrap().insert(p.clone(), item.clone());
                    }
                    let val = self.eval_expr(&body)?;
                    self.scopes.pop();
                    if self.is_truthy(&val) {
                        result.push(item.clone());
                    }
                }
                Ok(Value::List(result))
            }
            "map" => {
                let mut result = Vec::new();
                for item in &list {
                    self.scopes.push(HashMap::new());
                    if let Some(p) = params.first() {
                        self.scopes.last_mut().unwrap().insert(p.clone(), item.clone());
                    }
                    let val = self.eval_expr(&body)?;
                    self.scopes.pop();
                    result.push(val);
                }
                Ok(Value::List(result))
            }
            _ => Err(format!("unknown list method: {}", method)),
        }
    }

    fn builtin_method(&mut self, obj: Value, method: &str, args: Vec<Value>) -> Result<Value, String> {
        // Lambda-consuming methods
        if matches!(method, "filter" | "map") {
            return self.list_lambda_method(obj, method, args);
        }

        match (&obj, method) {
            (Value::List(l), "len") => Ok(Value::Int(l.len() as i64)),
            (Value::Str(s), "len") => Ok(Value::Int(s.len() as i64)),
            (Value::Str(s), "contains") => Ok(Value::Bool(s.contains(val_str(&args, 0, "contains")?.as_str()))),
            (Value::Str(s), "starts_with") => Ok(Value::Bool(s.starts_with(val_str(&args, 0, "starts_with")?.as_str()))),
            (Value::Str(s), "ends_with") => Ok(Value::Bool(s.ends_with(val_str(&args, 0, "ends_with")?.as_str()))),
            (Value::Str(s), "replace") => {
                let from = val_str(&args, 0, "replace")?;
                let to = val_str(&args, 1, "replace")?;
                Ok(Value::Str(s.replace(&from, &to)))
            }
            (Value::Str(s), "trim") => Ok(Value::Str(s.trim().into())),
            (Value::Str(s), "split") => {
                let sep = val_str(&args, 0, "split")?;
                Ok(Value::List(s.split(&sep).map(|p| Value::Str(p.into())).collect()))
            }
            (Value::Str(s), "find") => {
                let pat = val_str(&args, 0, "find")?;
                Ok(Value::Int(s.find(&pat).map(|i| i as i64).unwrap_or(-1)))
            }
            (Value::Str(s), "find_from") => {
                let pat = val_str(&args, 0, "find_from")?;
                let start = val_int(&args, 1, "find_from")?.max(0) as usize;
                let result = if start <= s.len() {
                    s[start..].find(&pat).map(|i| (i + start) as i64).unwrap_or(-1)
                } else { -1 };
                Ok(Value::Int(result))
            }
            (Value::Str(s), "slice") => {
                let start = val_int(&args, 0, "slice")?.max(0) as usize;
                let end = val_int(&args, 1, "slice")?.max(0) as usize;
                let start = start.min(s.len());
                let end = end.min(s.len()).max(start);
                Ok(Value::Str(s[start..end].into()))
            }
            (Value::Str(s), "slice_from") => {
                let start = val_int(&args, 0, "slice_from")?.max(0) as usize;
                let start = start.min(s.len());
                Ok(Value::Str(s[start..].into()))
            }
            (Value::Str(s), "split_lines") => {
                Ok(Value::List(s.lines().map(|l| Value::Str(l.into())).collect()))
            }
            (Value::Str(s), "to_string") => Ok(Value::Str(s.clone())),
            (Value::Str(s), "int") => {
                match s.trim().parse::<i64>() {
                    Ok(n) => Ok(ok_val(Value::Int(n))),
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            // List methods
            (Value::List(l), "is_empty") => Ok(Value::Bool(l.is_empty())),
            (Value::List(l), "first") => Ok(l.first().cloned().unwrap_or(Value::None)),
            (Value::List(l), "last") => Ok(l.last().cloned().unwrap_or(Value::None)),
            (Value::List(l), "sum") => {
                let sum: i64 = l.iter().map(|v| match v {
                    Value::Int(n) => *n,
                    _ => 0,
                }).sum();
                Ok(Value::Int(sum))
            }
            (Value::List(l), "join") => {
                let sep = val_str(&args, 0, "join")?;
                Ok(Value::Str(l.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep)))
            }
            // Int/Float methods
            (Value::Int(n), "abs") => Ok(Value::Int(n.abs())),
            (Value::Int(n), "to_string") => Ok(Value::Str(n.to_string())),
            (Value::Float(f), "abs") => Ok(Value::Float(f.abs())),
            (Value::Float(f), "to_string") => Ok(Value::Str(f.to_string())),
            // Row methods (column access)
            (Value::Struct { type_name, fields }, m)
                if type_name == "Row" && matches!(m, "int" | "string" | "bool") =>
            {
                let col = val_str(&args, 0, "row")?;
                let val = fields.get(&col).cloned().unwrap_or(Value::None);
                match m {
                    "int" => match val {
                        Value::Int(n) => Ok(Value::Int(n)),
                        Value::Str(s) => Ok(Value::Int(s.parse::<i64>().unwrap_or(0))),
                        _ => Ok(Value::Int(0)),
                    },
                    "string" => match val {
                        Value::Str(s) => Ok(Value::Str(s)),
                        v => Ok(Value::Str(v.to_string())),
                    },
                    "bool" => match val {
                        Value::Int(n) => Ok(Value::Bool(n != 0)),
                        Value::Bool(b) => Ok(Value::Bool(b)),
                        _ => Ok(Value::Bool(false)),
                    },
                    _ => unreachable!(),
                }
            }
            // Args.require(name)
            (Value::Struct { type_name, fields }, "require") if type_name == "Args" => {
                let param_name = val_str(&args, 0, "Args.require")?;
                match fields.get("_positional") {
                    Some(Value::List(pos)) if !pos.is_empty() =>
                        Ok(ok_val(pos[0].clone())),
                    _ => Ok(err_val(format!("missing argument: {}", param_name))),
                }
            }
            _ => Err(format!("unknown method .{} on {}", method, obj)),
        }
    }

    // ── Function calls ─────────────────────────────────────────────────

    fn call_fn(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            // ── I/O ────────────────────────────────────────────────────
            "print" => {
                if let Some(v) = args.first() { println!("{}", v); }
                Ok(Value::None)
            }
            "sleep" => {
                let ms = val_int(&args, 0, "sleep")?;
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Value::None)
            }

            // ── Testing ───────────────────────────────────────────────
            "assert" => {
                let val = args.first().unwrap_or(&Value::None);
                if self.is_truthy(val) { Ok(Value::None) }
                else { Err("assert failed".into()) }
            }
            "assert_eq" => {
                let a = args.get(0).cloned().unwrap_or(Value::None);
                let b = args.get(1).cloned().unwrap_or(Value::None);
                if values_equal(&a, &b) { Ok(Value::None) }
                else { Err(format!("assert_eq failed: got {}, expected {}", a, b)) }
            }

            // ── File system ────────────────────────────────────────────
            "read_file" => {
                let path = val_str(&args, 0, "read_file")?;
                match std::fs::read_to_string(&path) {
                    Ok(c) => Ok(ok_val(Value::Str(c))),
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            "write_file" => {
                let path = val_str(&args, 0, "write_file")?;
                let content = val_str(&args, 1, "write_file")?;
                match std::fs::write(&path, &content) {
                    Ok(_) => Ok(ok_val(Value::None)),
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            "list_dir" => {
                let path = val_str(&args, 0, "list_dir")?;
                match std::fs::read_dir(&path) {
                    Ok(entries) => {
                        let mut files: Vec<Value> = entries.flatten()
                            .filter_map(|e| e.file_name().to_str().map(|s| Value::Str(s.into())))
                            .collect();
                        files.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                        Ok(ok_val(Value::List(files)))
                    }
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            "create_dir" => {
                let path = val_str(&args, 0, "create_dir")?;
                match std::fs::create_dir_all(&path) {
                    Ok(_) => Ok(ok_val(Value::None)),
                    Err(e) => Ok(err_val(e.to_string())),
                }
            }
            "path_stem" => {
                let path = val_str(&args, 0, "path_stem")?;
                let stem = std::path::Path::new(&path)
                    .file_stem().and_then(|s| s.to_str())
                    .unwrap_or(&path);
                Ok(Value::Str(stem.into()))
            }

            // ── String functions ───────────────────────────────────────
            "starts_with" => {
                let (text, prefix) = (val_str(&args, 0, "starts_with")?, val_str(&args, 1, "starts_with")?);
                Ok(Value::Bool(text.starts_with(&prefix)))
            }
            "ends_with" => {
                let (text, suffix) = (val_str(&args, 0, "ends_with")?, val_str(&args, 1, "ends_with")?);
                Ok(Value::Bool(text.ends_with(&suffix)))
            }
            "contains" => {
                let (text, sub) = (val_str(&args, 0, "contains")?, val_str(&args, 1, "contains")?);
                Ok(Value::Bool(text.contains(&sub)))
            }
            "replace" => {
                let (text, from, to) = (val_str(&args, 0, "replace")?, val_str(&args, 1, "replace")?, val_str(&args, 2, "replace")?);
                Ok(Value::Str(text.replace(&from, &to)))
            }
            "find" => {
                let (text, pat) = (val_str(&args, 0, "find")?, val_str(&args, 1, "find")?);
                Ok(Value::Int(text.find(&pat).map(|i| i as i64).unwrap_or(-1)))
            }
            "find_from" => {
                let text = val_str(&args, 0, "find_from")?;
                let pat = val_str(&args, 1, "find_from")?;
                let start = val_int(&args, 2, "find_from")?.max(0) as usize;
                let result = if start <= text.len() {
                    text[start..].find(&pat).map(|i| (i + start) as i64).unwrap_or(-1)
                } else { -1 };
                Ok(Value::Int(result))
            }
            "slice_from" => {
                let text = val_str(&args, 0, "slice_from")?;
                let start = val_int(&args, 1, "slice_from")?.max(0) as usize;
                let start = start.min(text.len());
                Ok(Value::Str(text[start..].into()))
            }
            "slice_range" => {
                let text = val_str(&args, 0, "slice_range")?;
                let start = val_int(&args, 1, "slice_range")?.max(0) as usize;
                let end = val_int(&args, 2, "slice_range")?.max(0) as usize;
                let start = start.min(text.len());
                let end = end.min(text.len()).max(start);
                Ok(Value::Str(text[start..end].into()))
            }
            "split_lines" => {
                let text = val_str(&args, 0, "split_lines")?;
                Ok(Value::List(text.lines().map(|l| Value::Str(l.into())).collect()))
            }
            "str_len" => {
                let text = val_str(&args, 0, "str_len")?;
                Ok(Value::Int(text.len() as i64))
            }
            "trim" => {
                let text = val_str(&args, 0, "trim")?;
                Ok(Value::Str(text.trim().into()))
            }

            // ── Collection / conversion ────────────────────────────────
            "len" => match args.first() {
                Some(Value::List(l)) => Ok(Value::Int(l.len() as i64)),
                Some(Value::Str(s)) => Ok(Value::Int(s.len() as i64)),
                _ => Err("len: expected list or string".into()),
            },
            "str" => Ok(Value::Str(args.first().map(|v| v.to_string()).unwrap_or_default())),
            "int" => match args.first() {
                Some(Value::Str(s)) => s.trim().parse::<i64>().map(Value::Int).map_err(|e| format!("int: {}", e)),
                Some(Value::Float(f)) => Ok(Value::Int(*f as i64)),
                Some(Value::Int(n)) => Ok(Value::Int(*n)),
                _ => Err("int: expected string or number".into()),
            },
            "float" => match args.first() {
                Some(Value::Str(s)) => s.trim().parse::<f64>().map(Value::Float).map_err(|e| format!("float: {}", e)),
                Some(Value::Int(n)) => Ok(Value::Float(*n as f64)),
                Some(Value::Float(f)) => Ok(Value::Float(*f)),
                _ => Err("float: expected string or number".into()),
            },
            "type" => Ok(Value::Str(match args.first() {
                Some(Value::Int(_)) => "Int",
                Some(Value::Float(_)) => "Float",
                Some(Value::Str(_)) => "String",
                Some(Value::Bool(_)) => "Bool",
                Some(Value::List(_)) => "List",
                Some(Value::None) | None => "None",
                Some(Value::Struct { type_name, .. }) => return Ok(Value::Str(type_name.clone())),
                Some(Value::Variant { name, .. }) => return Ok(Value::Str(name.clone())),
                Some(Value::Channel(_)) => "Channel",
                Some(Value::Lambda { .. }) => "Lambda",
                Some(Value::Db(_)) => "Db",
                Some(Value::Type(n)) => return Ok(Value::Str(n.clone())),
            }.into())),
            "push" => {
                if args.len() == 2 {
                    if let Value::List(mut l) = args[0].clone() {
                        l.push(args[1].clone());
                        Ok(Value::List(l))
                    } else { Err("push: first arg must be list".into()) }
                } else { Err("push: expected 2 args".into()) }
            }
            "range" => match (args.get(0), args.get(1)) {
                (Some(Value::Int(start)), Some(Value::Int(end))) =>
                    Ok(Value::List((*start..*end).map(Value::Int).collect())),
                (Some(Value::Int(end)), None) =>
                    Ok(Value::List((0..*end).map(Value::Int).collect())),
                _ => Err("range: expected int args".into()),
            },

            // ── User-defined functions ─────────────────────────────────
            _ => {
                let def = self.fns.get(name).cloned()
                    .ok_or_else(|| format!("undefined function: {}", name))?;
                if args.len() != def.params.len() {
                    return Err(format!("{}: expected {} args, got {}",
                        name, def.params.len(), args.len()));
                }
                let global = self.scopes[0].clone();
                let saved = std::mem::replace(&mut self.scopes, vec![global, HashMap::new()]);
                for (param, val) in def.params.iter().zip(args) {
                    self.scopes[1].insert(param.clone(), val);
                }
                let result = match self.exec_block(&def.body)? {
                    Signal::Return(v) => v,
                    Signal::None => Value::None,
                };
                self.scopes = saved;
                Ok(result)
            }
        }
    }

    // ── Operators ──────────────────────────────────────────────────────

    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::None => false,
            _ => true,
        }
    }

    fn eval_binop(&self, left: &Value, op: &str, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => match op {
                "+" => Ok(Value::Int(a + b)),
                "-" => Ok(Value::Int(a - b)),
                "*" => Ok(Value::Int(a * b)),
                "/" => { if *b == 0 { return Err("division by zero".into()); } Ok(Value::Int(a / b)) }
                "%" => { if *b == 0 { return Err("division by zero".into()); } Ok(Value::Int(a % b)) }
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                ">" => Ok(Value::Bool(a > b)),
                "<" => Ok(Value::Bool(a < b)),
                ">=" => Ok(Value::Bool(a >= b)),
                "<=" => Ok(Value::Bool(a <= b)),
                _ => Err(format!("unknown op: {}", op)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "+" => Ok(Value::Float(a + b)),
                "-" => Ok(Value::Float(a - b)),
                "*" => Ok(Value::Float(a * b)),
                "/" => Ok(Value::Float(a / b)),
                "%" => Ok(Value::Float(a % b)),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                ">" => Ok(Value::Bool(a > b)),
                "<" => Ok(Value::Bool(a < b)),
                ">=" => Ok(Value::Bool(a >= b)),
                "<=" => Ok(Value::Bool(a <= b)),
                _ => Err(format!("unknown op: {}", op)),
            },
            (Value::Int(a), Value::Float(_)) => self.eval_binop(&Value::Float(*a as f64), op, right),
            (Value::Float(_), Value::Int(b)) => self.eval_binop(left, op, &Value::Float(*b as f64)),
            (Value::Str(a), Value::Str(b)) => match op {
                "+" => Ok(Value::Str(format!("{}{}", a, b))),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                _ => Err(format!("string op not supported: {}", op)),
            },
            (Value::Bool(a), Value::Bool(b)) => match op {
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                _ => Err(format!("bool op not supported: {}", op)),
            },
            _ => Err(format!("type mismatch: {} {} {}", left, op, right)),
        }
    }
}
