use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use crate::ast::*;

struct Ctx {
    structs: HashSet<String>,
    struct_fields: HashMap<String, Vec<String>>,
    variant_to_enum: HashMap<String, String>,
    fn_params: HashMap<String, Vec<FnParam>>,
    async_fns: HashSet<String>,
    line_starts: Vec<usize>,
    filename: String,
    struct_vars: RefCell<HashSet<String>>,
}

impl Ctx {
    fn new(program: &Program, source: &str, filename: &str) -> Self {
        let mut ctx = Ctx {
            structs: HashSet::new(), struct_fields: HashMap::new(),
            variant_to_enum: HashMap::new(),
            fn_params: HashMap::new(), async_fns: HashSet::new(),
            line_starts: compute_line_starts(source), filename: filename.to_string(),
            struct_vars: RefCell::new(HashSet::new()),
        };
        for stmt in &program.statements {
            match stmt {
                Stmt::StructDef { name, fields, .. } => {
                    ctx.structs.insert(name.clone());
                    ctx.struct_fields.insert(name.clone(), fields.iter().map(|f| f.name.clone()).collect());
                }
                Stmt::TypeDef { name, variants, .. } => {
                    for v in variants { ctx.variant_to_enum.insert(v.name.clone(), name.clone()); }
                }
                Stmt::FnDef { name, params, .. } => {
                    ctx.fn_params.insert(name.clone(), params.clone());
                }
                _ => {}
            }
        }
        ctx.async_fns = compute_async_fns(program);
        // Pre-populate struct_vars from top-level assignments
        for stmt in &program.statements {
            match stmt {
                Stmt::Assignment { name, value, .. } => {
                    if is_struct_creating_expr(value, &ctx) {
                        ctx.struct_vars.borrow_mut().insert(name.clone());
                    }
                }
                _ => {}
            }
        }
        ctx
    }

    fn line_of(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, c) in source.char_indices() {
        if c == '\n' { starts.push(i + 1); }
    }
    starts
}

// ─── Async analysis ──────────────────────────────────────

fn compute_async_fns(program: &Program) -> HashSet<String> {
    let mut fn_bodies: HashMap<&str, &[Stmt]> = HashMap::new();
    for stmt in &program.statements {
        if let Stmt::FnDef { name, body, .. } = stmt {
            fn_bodies.insert(name, body);
        }
    }
    // Seed: functions that directly use async runtime operations
    let mut async_fns = HashSet::new();
    for (&name, body) in &fn_bodies {
        if stmts_need_async(body) { async_fns.insert(name.to_string()); }
    }
    // Fixed-point: propagate through call graph
    loop {
        let mut changed = false;
        for (&name, body) in &fn_bodies {
            if !async_fns.contains(name) && stmts_call_any_async(body, &async_fns) {
                async_fns.insert(name.to_string());
                changed = true;
            }
        }
        if !changed { break; }
    }
    async_fns
}

fn stmts_need_async(stmts: &[Stmt]) -> bool { stmts.iter().any(stmt_needs_async) }
fn stmt_needs_async(s: &Stmt) -> bool {
    match s {
        Stmt::ExprStmt { expr, .. } | Stmt::Assignment { value: expr, .. } => expr_needs_async(expr),
        Stmt::Return { value: Some(e), .. } => expr_needs_async(e),
        Stmt::If { condition, body, elifs, else_body, .. } =>
            expr_needs_async(condition) || stmts_need_async(body)
            || elifs.iter().any(|(c, b)| expr_needs_async(c) || stmts_need_async(b))
            || else_body.as_ref().map_or(false, |b| stmts_need_async(b)),
        Stmt::ForLoop { iter, body, .. } => expr_needs_async(iter) || stmts_need_async(body),
        Stmt::Loop { body, .. } | Stmt::WhileLoop { body, .. } => stmts_need_async(body),
        Stmt::Match { expr, arms, .. } => expr_needs_async(expr) || arms.iter().any(|a| stmt_needs_async(&a.body)),
        _ => false,
    }
}
fn expr_needs_async(e: &Expr) -> bool {
    match e {
        Expr::FunctionCall { name, args, .. } => is_async_function(name) || args.iter().any(expr_needs_async),
        Expr::MethodCall { object, method, args, .. } => is_async_method(method) || expr_needs_async(object) || args.iter().any(expr_needs_async),
        Expr::BinaryOp { left, right, .. } => expr_needs_async(left) || expr_needs_async(right),
        Expr::UnaryOp { expr, .. } | Expr::PostfixOp { expr, .. } => expr_needs_async(expr),
        Expr::FieldAccess { object, .. } => expr_needs_async(object),
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_needs_async(v)),
        _ => false,
    }
}

fn stmts_call_any_async(stmts: &[Stmt], af: &HashSet<String>) -> bool { stmts.iter().any(|s| stmt_calls_async(s, af)) }
fn stmt_calls_async(s: &Stmt, af: &HashSet<String>) -> bool {
    match s {
        Stmt::ExprStmt { expr, .. } | Stmt::Assignment { value: expr, .. } => expr_calls_async(expr, af),
        Stmt::Return { value: Some(e), .. } => expr_calls_async(e, af),
        Stmt::If { condition, body, elifs, else_body, .. } =>
            expr_calls_async(condition, af) || stmts_call_any_async(body, af)
            || elifs.iter().any(|(c, b)| expr_calls_async(c, af) || stmts_call_any_async(b, af))
            || else_body.as_ref().map_or(false, |b| stmts_call_any_async(b, af)),
        Stmt::ForLoop { iter, body, .. } => expr_calls_async(iter, af) || stmts_call_any_async(body, af),
        Stmt::Loop { body, .. } | Stmt::WhileLoop { body, .. } => stmts_call_any_async(body, af),
        Stmt::Match { expr, arms, .. } => expr_calls_async(expr, af) || arms.iter().any(|a| stmt_calls_async(&a.body, af)),
        _ => false,
    }
}
fn expr_calls_async(e: &Expr, af: &HashSet<String>) -> bool {
    match e {
        Expr::FunctionCall { name, args, .. } => af.contains(name.as_str()) || args.iter().any(|a| expr_calls_async(a, af)),
        Expr::MethodCall { object, args, .. } => expr_calls_async(object, af) || args.iter().any(|a| expr_calls_async(a, af)),
        Expr::BinaryOp { left, right, .. } => expr_calls_async(left, af) || expr_calls_async(right, af),
        Expr::UnaryOp { expr, .. } | Expr::PostfixOp { expr, .. } => expr_calls_async(expr, af),
        Expr::FieldAccess { object, .. } => expr_calls_async(object, af),
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_calls_async(v, af)),
        _ => false,
    }
}

fn is_struct_creating_expr(expr: &Expr, ctx: &Ctx) -> bool {
    match expr {
        Expr::StructInit { name, .. } => ctx.structs.contains(name),
        // Static method call on struct type: Counter.new() → likely returns struct
        Expr::MethodCall { object, .. } => {
            if let Expr::Identifier { name, .. } = object.as_ref() {
                ctx.structs.contains(name.as_str())
            } else { false }
        }
        _ => false,
    }
}

fn is_struct_var_ident(expr: &Expr, ctx: &Ctx) -> bool {
    if let Expr::Identifier { name, .. } = expr {
        ctx.struct_vars.borrow().contains(name.as_str())
    } else { false }
}

// Find field access on a parameter in expressions (for type inference)
fn expr_accesses_field_on(param: &str, expr: &Expr) -> Option<String> {
    match expr {
        Expr::FieldAccess { object, field, .. } => {
            if let Expr::Identifier { name, .. } = object.as_ref() {
                if name == param { return Some(field.clone()); }
            }
            expr_accesses_field_on(param, object)
        }
        Expr::MethodCall { object, args, .. } => {
            expr_accesses_field_on(param, object)
                .or_else(|| args.iter().find_map(|a| expr_accesses_field_on(param, a)))
        }
        Expr::FunctionCall { args, .. } => args.iter().find_map(|a| expr_accesses_field_on(param, a)),
        Expr::BinaryOp { left, right, .. } => {
            expr_accesses_field_on(param, left).or_else(|| expr_accesses_field_on(param, right))
        }
        Expr::UnaryOp { expr: e, .. } | Expr::PostfixOp { expr: e, .. } => expr_accesses_field_on(param, e),
        Expr::Lambda { body, .. } => expr_accesses_field_on(param, body),
        Expr::List { elements, .. } => elements.iter().find_map(|e| expr_accesses_field_on(param, e)),
        Expr::StructInit { fields, .. } => fields.iter().find_map(|(_, v)| expr_accesses_field_on(param, v)),
        Expr::StringLiteral { parts, .. } => parts.iter().find_map(|p| match p {
            StringPart::Interpolation(e) => expr_accesses_field_on(param, e),
            _ => None,
        }),
        _ => None,
    }
}

fn stmts_access_field_on(param: &str, stmts: &[Stmt]) -> Option<String> {
    for stmt in stmts {
        let result = match stmt {
            Stmt::ExprStmt { expr, .. } => expr_accesses_field_on(param, expr),
            Stmt::Assignment { value, .. } => expr_accesses_field_on(param, value),
            Stmt::Return { value: Some(e), .. } => expr_accesses_field_on(param, e),
            Stmt::MutAssign { object, value, .. } =>
                expr_accesses_field_on(param, object).or_else(|| expr_accesses_field_on(param, value)),
            Stmt::If { condition, body, elifs, else_body, .. } =>
                expr_accesses_field_on(param, condition)
                    .or_else(|| stmts_access_field_on(param, body))
                    .or_else(|| elifs.iter().find_map(|(c, b)|
                        expr_accesses_field_on(param, c).or_else(|| stmts_access_field_on(param, b))))
                    .or_else(|| else_body.as_ref().and_then(|b| stmts_access_field_on(param, b))),
            Stmt::ForLoop { iter, body, .. } =>
                expr_accesses_field_on(param, iter).or_else(|| stmts_access_field_on(param, body)),
            Stmt::Loop { body, .. } | Stmt::WhileLoop { body, .. } => stmts_access_field_on(param, body),
            Stmt::Match { expr, arms, .. } =>
                expr_accesses_field_on(param, expr)
                    .or_else(|| arms.iter().find_map(|a| match &a.body {
                        s => { let ss = vec![s.clone()]; stmts_access_field_on(param, &ss) }
                    })),
            _ => None,
        };
        if result.is_some() { return result; }
    }
    None
}

fn map_type(t: &str) -> String {
    // Handle generic types like Maybe[Int], List[String], Phantom[T]
    if let Some(start) = t.find('[') {
        let end = t.rfind(']').unwrap_or(t.len());
        let base = &t[..start];
        let inner = &t[start+1..end];
        
        let mapped_base = match base {
            "Maybe" => "Maybe",
            "List" => "Vec",
            "Phantom" => "std::marker::PhantomData",
            other => other,
        };
        
        let mapped_inner = map_type(inner);
        return format!("{}<{}>", mapped_base, mapped_inner);
    }
    
    // Simple types
    match t {
        "Int" => "i64".to_string(),
        "Float" => "f64".to_string(),
        "String" => "String".to_string(),
        "Bool" => "bool".to_string(),
        "Channel" => "Chan".to_string(),
        "Response" => "HttpResponse".to_string(),
        o => o.to_string(),
    }
}

fn map_param_type(t: &str, structs: &HashSet<String>, is_mut: bool) -> String {
    match t {
        "Int" => "i64".into(), "Float" => "f64".into(), "Bool" => "bool".into(),
        "String" => "&str".into(), "Db" => "&Db".into(),
        "Channel" => "Chan".into(),
        other => {
            if structs.contains(other) {
                if is_mut { format!("&mut {}", other) } else { format!("&{}", other) }
            } else {
                format!("&{}", other)
            }
        }
    }
}

fn is_ref_type(t: &str) -> bool {
    !matches!(t, "Int" | "Float" | "Bool" | "Channel")
}

fn map_return_type(t: &str, _structs: &HashSet<String>) -> String {
    map_type(t)
}

fn collect_free_vars(expr: &Expr) -> Vec<String> {
    let mut vars = Vec::new();
    match expr {
        Expr::Identifier { name, .. } => vars.push(name.clone()),
        Expr::FunctionCall { args, .. } => {
            for a in args { vars.extend(collect_free_vars(a)); }
        }
        Expr::MethodCall { object, args, .. } => {
            vars.extend(collect_free_vars(object));
            for a in args { vars.extend(collect_free_vars(a)); }
        }
        Expr::Lambda { body, params, .. } => {
            let inner = collect_free_vars(body);
            for v in inner { if !params.contains(&v) { vars.push(v); } }
        }
        Expr::BinaryOp { left, right, .. } => {
            vars.extend(collect_free_vars(left));
            vars.extend(collect_free_vars(right));
        }
        Expr::UnaryOp { expr, .. } | Expr::PostfixOp { expr, .. } => {
            vars.extend(collect_free_vars(expr));
        }
        Expr::FieldAccess { object, .. } => vars.extend(collect_free_vars(object)),
        _ => {}
    }
    vars
}

// Check if any expression in stmts uses postfix ?
fn uses_qmark(stmts: &[Stmt]) -> bool { stmts.iter().any(|s| stmt_has_qmark(s)) }
fn stmt_has_qmark(s: &Stmt) -> bool {
    match s {
        Stmt::ExprStmt { expr, .. } | Stmt::Assignment { value: expr, .. } => expr_has_qmark(expr),
        Stmt::Return { value: Some(e), .. } => expr_has_qmark(e),
        Stmt::If { condition, body, elifs, else_body, .. } =>
            expr_has_qmark(condition) || uses_qmark(body)
            || elifs.iter().any(|(c, b)| expr_has_qmark(c) || uses_qmark(b))
            || else_body.as_ref().map_or(false, |b| uses_qmark(b)),
        Stmt::ForLoop { iter, body, .. } => expr_has_qmark(iter) || uses_qmark(body),
        Stmt::WhileLoop { condition, body, .. } => expr_has_qmark(condition) || uses_qmark(body),
        Stmt::Loop { body, .. } => uses_qmark(body),
        Stmt::Match { expr, arms, .. } => expr_has_qmark(expr) || arms.iter().any(|a| stmt_has_qmark(&a.body)),
        _ => false,
    }
}
fn expr_has_qmark(e: &Expr) -> bool {
    match e {
        Expr::PostfixOp { op, expr, .. } => op == "?" || expr_has_qmark(expr),
        Expr::MethodCall { object, args, .. } => expr_has_qmark(object) || args.iter().any(expr_has_qmark),
        Expr::FunctionCall { args, .. } => args.iter().any(expr_has_qmark),
        Expr::BinaryOp { left, right, .. } => expr_has_qmark(left) || expr_has_qmark(right),
        Expr::UnaryOp { expr, .. } => expr_has_qmark(expr),
        Expr::FieldAccess { object, .. } => expr_has_qmark(object),
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_has_qmark(v)),
        _ => false,
    }
}

fn has_return_value(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return { value: Some(_), .. } => true,
        Stmt::If { body, elifs, else_body, .. } =>
            has_return_value(body) || elifs.iter().any(|(_, b)| has_return_value(b))
            || else_body.as_ref().map_or(false, |b| has_return_value(b)),
        Stmt::ForLoop { body, .. } | Stmt::Loop { body, .. } | Stmt::WhileLoop { body, .. } => has_return_value(body),
        Stmt::Match { arms, .. } => arms.iter().any(|a| matches!(&a.body, Stmt::Return { value: Some(_), .. })),
        _ => false,
    })
}

fn infer_param_type(param: &str, body: &[Stmt], ctx: &Ctx, return_type: Option<&str>, is_mut: bool) -> String {
    // HTTP handler: fn f(request) -> Response → param is HttpRequest
    if return_type == Some("Response") {
        return "HttpRequest".into();
    }
    for stmt in body {
        if let Stmt::Match { expr, arms, .. } = stmt {
            if let Expr::Identifier { name, .. } = expr {
                if name == param {
                    if let Some(arm) = arms.first() {
                        if let MatchPattern::Variant { name: vn, .. } = &arm.pattern {
                            if let Some(en) = ctx.variant_to_enum.get(vn) { return en.clone(); }
                        }
                    }
                }
            }
        }
    }
    // Struct inference: param.field access → find which struct has that field
    if let Some(field_name) = stmts_access_field_on(param, body) {
        for (sname, sfields) in &ctx.struct_fields {
            if sfields.contains(&field_name) {
                return if is_mut { format!("&mut {}", sname) } else { format!("&{}", sname) };
            }
        }
    }
    "i64".to_string()
}

fn stmt_offset(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Assignment { span, .. } | Stmt::ExprStmt { span, .. }
        | Stmt::FnDef { span, .. } | Stmt::ForLoop { span, .. }
        | Stmt::If { span, .. } | Stmt::Return { span, .. }
        | Stmt::StructDef { span, .. } | Stmt::TypeDef { span, .. }
        | Stmt::Match { span, .. } | Stmt::MutAssign { span, .. }
        | Stmt::Spawn { span, .. } | Stmt::Loop { span, .. }
        | Stmt::MemoryDecl { span, .. } | Stmt::UseDecl { span, .. }
        | Stmt::TraitDef { span, .. } | Stmt::ImplBlock { span, .. }
        | Stmt::WhileLoop { span, .. } | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        => span.start,
    }
}

fn infer_method_ret(body: &[Stmt], target: &str, _structs: &HashSet<String>) -> String {
    for stmt in body {
        if let Stmt::Return { value: Some(Expr::StructInit { name, .. }), .. } = stmt {
            if name == target {
                return target.to_string();
            }
        }
    }
    "i64".to_string()
}

pub fn generate(program: &Program, source: &str, filename: &str, rs_modules: &[String], ext_fn_params: &HashMap<String, Vec<FnParam>>) -> Result<String, String> {
    let mut ctx = Ctx::new(program, source, filename);
    for (name, params) in ext_fn_params {
        ctx.fn_params.entry(name.clone()).or_insert_with(|| params.clone());
    }
    validate_spawn_safety(program, &ctx)?;

    let mut out = String::new();
    out.push_str("#![allow(unused_mut, unused_variables, dead_code, unused_imports)]\n");
    out.push_str("use u_runtime::*;\n");

    // Emit mod declarations for all modules (.rs and .u-transpiled)
    for m in rs_modules {
        out.push_str("mod "); out.push_str(m); out.push_str(";\n");
    }

    // Emit use statements for module imports
    for stmt in &program.statements {
        if let Stmt::UseDecl { path, imports, .. } = stmt {
            // Flat module name: utils.strings → utils_strings, math → math
            let flat_name = path.replace('.', "_");
            if rs_modules.iter().any(|m| m == path || m == &flat_name) {
                out.push_str("use "); out.push_str(&flat_name); out.push_str("::");
                if imports.len() == 1 {
                    out.push_str(&imports[0]);
                } else {
                    out.push('{');
                    out.push_str(&imports.join(", "));
                    out.push('}');
                }
                out.push_str(";\n");
            }
        }
    }

    out.push('\n');

    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. }
            | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. } => {
                let mut decl = HashSet::new();
                gen_stmt(stmt, &mut out, 0, &ctx, false, &mut decl);
                out.push('\n');
            }
            _ => {}
        }
    }

    out.push_str("#[tokio::main]\nasync fn main() {\n");
    out.push_str("    std::panic::set_hook(Box::new(|info| {\n");
    out.push_str("        if std::thread::current().name() == Some(\"main\") {\n");
    out.push_str("            eprintln!(\"{}\", info);\n");
    out.push_str("        } else {\n");
    out.push_str("            eprintln!(\"goroutine panic: {}\", info);\n");
    out.push_str("        }\n");
    out.push_str("    }));\n");
    out.push_str("    if let Err(e) = _u_main().await { eprintln!(\"Ошибка: {}\", e); std::process::exit(1); }\n");
    out.push_str("}\n\n");
    out.push_str("async fn _u_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {\n");
    let mut main_declared = HashSet::new();
    for stmt in &program.statements {
        if !matches!(stmt, Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. }
            | Stmt::MemoryDecl { .. } | Stmt::UseDecl { .. }
            | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. }) {
            gen_stmt(stmt, &mut out, 1, &ctx, true, &mut main_declared);
        }
    }
    out.push_str("    Ok(())\n}\n");
    Ok(out)
}

/// Generate a module file (no main, just declarations)
pub fn generate_module(program: &Program, source: &str, filename: &str) -> Result<String, String> {
    let ctx = Ctx::new(program, source, filename);

    let mut out = String::new();
    out.push_str("#![allow(unused_mut, unused_variables, dead_code, unused_imports)]\n");
    out.push_str("use u_runtime::*;\n\n");

    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. }
            | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. } => {
                let mut decl = HashSet::new();
                gen_stmt(stmt, &mut out, 0, &ctx, false, &mut decl);
                out.push('\n');
            }
            _ => {}
        }
    }
    Ok(out)
}

fn validate_spawn_safety(program: &Program, ctx: &Ctx) -> Result<(), String> {
    for stmt in &program.statements {
        validate_stmt_spawn(stmt, ctx)?;
    }
    Ok(())
}

fn validate_stmt_spawn(stmt: &Stmt, ctx: &Ctx) -> Result<(), String> {
    match stmt {
        Stmt::Spawn { expr, .. } => {
            if let Expr::FunctionCall { name, .. } = expr {
                if let Some(params) = ctx.fn_params.get(name.as_str()) {
                    if let Some(p) = params.iter().find(|p| p.is_mut) {
                        return Err(format!(
                            "error: cannot mutate external variable in spawn — function '{}' has 'mut {}' parameter. Use a channel (.send) instead.",
                            name, p.name
                        ));
                    }
                }
            }
        }
        Stmt::FnDef { body, .. } | Stmt::ForLoop { body, .. } | Stmt::Loop { body, .. }
        | Stmt::WhileLoop { body, .. } => {
            for s in body { validate_stmt_spawn(s, ctx)?; }
        }
        Stmt::ImplBlock { methods, .. } => {
            for m in methods { validate_stmt_spawn(m, ctx)?; }
        }
        Stmt::If { body, elifs, else_body, .. } => {
            for s in body { validate_stmt_spawn(s, ctx)?; }
            for (_, b) in elifs { for s in b { validate_stmt_spawn(s, ctx)?; } }
            if let Some(eb) = else_body { for s in eb { validate_stmt_spawn(s, ctx)?; } }
        }
        Stmt::Match { arms, .. } => {
            for arm in arms { validate_stmt_spawn(&arm.body, ctx)?; }
        }
        _ => {}
    }
    Ok(())
}

fn gen_stmts(stmts: &[Stmt], out: &mut String, indent: usize, ctx: &Ctx, result_fn: bool, declared: &mut HashSet<String>) {
    for stmt in stmts { gen_stmt(stmt, out, indent, ctx, result_fn, declared); }
}

fn is_async_function(name: &str) -> bool {
    matches!(name, "sleep" | "serve")
}

fn is_async_method(method: &str) -> bool {
    matches!(method, "recv" | "recv_timeout" | "accept" | "listen" | "respond" | "path")
}

fn runtime_param_types(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "read_file" => Some(&["&str"]),
        "write_file" => Some(&["&str", "&str"]),
        "list_dir" => Some(&["&str"]),
        "create_dir" => Some(&["&str"]),
        "mime_type" => Some(&["&str"]),
        "error" => Some(&["&str"]),
        "starts_with" => Some(&["&str", "&str"]),
        "ends_with" => Some(&["&str", "&str"]),
        "contains" => Some(&["&str", "&str"]),
        "replace" => Some(&["&str", "&str", "&str"]),
        "find" => Some(&["&str", "&str"]),
        "find_from" => Some(&["&str", "&str", "i64"]),
        "slice_from" => Some(&["&str", "i64"]),
        "slice_range" => Some(&["&str", "i64", "i64"]),
        "split_lines" => Some(&["&str"]),
        "str_len" => Some(&["&str"]),
        "trim" => Some(&["&str"]),
        "path_stem" => Some(&["&str"]),
        "copy_file" => Some(&["&str", "&str"]),
        "copy_dir" => Some(&["&str", "&str"]),
        "is_dir" => Some(&["&str"]),
        "range" => Some(&["i64"]),
        "parse_json" => Some(&["&str"]),
        "to_json" => Some(&["&ref"]),
        _ => None,
    }
}

fn is_plain_string(expr: &Expr) -> bool {
    matches!(expr, Expr::StringLiteral { parts, .. } if !parts.iter().any(|p| matches!(p, StringPart::Interpolation(_))))
}

fn gen_stmt(stmt: &Stmt, out: &mut String, indent: usize, ctx: &Ctx, result_fn: bool, declared: &mut HashSet<String>) {
    if matches!(stmt, Stmt::MemoryDecl { .. } | Stmt::UseDecl { .. }) {
        return;
    }
    let pad = "    ".repeat(indent);
    // Emit source line comment for error mapping
    if !ctx.filename.is_empty() {
        let line = ctx.line_of(stmt_offset(stmt));
        if line > 0 {
            out.push_str(&pad);
            out.push_str("// line:");
            out.push_str(&line.to_string());
            out.push('\n');
        }
    }
    out.push_str(&pad);
    match stmt {
        Stmt::StructDef { name, type_params, fields, is_pub, .. } => {
            // Skip Phantom - we use std::marker::PhantomData directly
            if name == "Phantom" {
                return;
            }
            out.push_str("#[derive(Debug, Clone)]\n");
            if *is_pub { out.push_str(&pad); out.push_str("pub "); } else { out.push_str(&pad); }
            out.push_str("struct ");
            out.push_str(name);
            if !type_params.is_empty() {
                out.push('<');
                for (i, tp) in type_params.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(tp);
                }
                out.push('>');
            }
            out.push_str(" {\n");
            for f in fields {
                out.push_str(&pad); out.push_str("    ");
                if *is_pub { out.push_str("pub "); }
                out.push_str(&f.name); out.push_str(": "); out.push_str(&map_type(&f.type_name)); out.push_str(",\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::TypeDef { name, type_params, variants, is_pub, .. } => {
            out.push_str("#[derive(Debug, Clone)]\n");
            if *is_pub { out.push_str(&pad); out.push_str("pub "); } else { out.push_str(&pad); }
            out.push_str("enum ");
            out.push_str(name);
            if !type_params.is_empty() {
                out.push('<');
                for (i, tp) in type_params.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(tp);
                }
                out.push('>');
            }
            out.push_str(" {\n");
            for v in variants {
                out.push_str(&pad); out.push_str("    "); out.push_str(&v.name);
                if !v.fields.is_empty() {
                    out.push('(');
                    for (i, f) in v.fields.iter().enumerate() {
                        if i > 0 { out.push_str(", "); }
                        out.push_str(&map_type(&f.type_name));
                    }
                    out.push(')');
                }
                out.push_str(",\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::Assignment { name, value, .. } => {
            // Track struct variables
            if is_struct_creating_expr(value, ctx) {
                ctx.struct_vars.borrow_mut().insert(name.clone());
            }
            if declared.contains(name.as_str()) {
                out.push_str(name); out.push_str(" = ");
            } else {
                declared.insert(name.clone());
                out.push_str("let mut "); out.push_str(name); out.push_str(" = ");
            }
            gen_expr(value, out, ctx);
            if is_plain_string(value) { out.push_str(".to_string()"); }
            out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            gen_expr(expr, out, ctx); out.push_str(";\n");
        }
        Stmt::FnDef { name, params, return_type, body, is_pub, .. } => {
            let fn_uses_q = uses_qmark(body);
            let has_ret = has_return_value(body);
            // Save struct_vars snapshot for function scope
            let saved_struct_vars: HashSet<String> = ctx.struct_vars.borrow().clone();
            // Detect struct params and add to struct_vars
            let mut struct_params = Vec::new();
            for p in params.iter() {
                if p.name == "self" { continue; }
                let is_struct = if let Some(ref t) = p.type_ann {
                    ctx.structs.contains(t.as_str())
                } else {
                    stmts_access_field_on(&p.name, body).map(|f|
                        ctx.struct_fields.values().any(|fields| fields.contains(&f))
                    ).unwrap_or(false)
                };
                if is_struct {
                    ctx.struct_vars.borrow_mut().insert(p.name.clone());
                    struct_params.push(p.name.clone());
                }
            }
            if *is_pub { out.push_str("pub "); }
            if ctx.async_fns.contains(name) { out.push_str("async "); }
            out.push_str("fn "); out.push_str(name); out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&p.name); out.push_str(": ");
                if let Some(ref t) = p.type_ann {
                    out.push_str(&map_param_type(t, &ctx.structs, p.is_mut));
                } else {
                    out.push_str(&infer_param_type(&p.name, body, ctx, return_type.as_deref(), p.is_mut));
                }
            }
            out.push(')');
            if fn_uses_q {
                let ret = return_type.as_deref()
                    .map(|t| map_return_type(t, &ctx.structs))
                    .unwrap_or_else(|| (if has_ret { "i64" } else { "()" }).to_string());
                out.push_str(" -> Result<"); out.push_str(&ret); out.push_str(", Box<dyn std::error::Error + Send + Sync>>");
            } else if has_ret {
                let ret = return_type.as_deref()
                    .map(|t| map_return_type(t, &ctx.structs))
                    .unwrap_or_else(|| "i64".to_string());
                out.push_str(" -> "); out.push_str(&ret);
            }
            out.push_str(" {\n");
            let mut fn_declared = HashSet::new();
            gen_stmts(body, out, indent + 1, ctx, fn_uses_q, &mut fn_declared);
            if fn_uses_q && !has_ret { out.push_str(&pad); out.push_str("    Ok(())\n"); }
            // Restore struct_vars
            *ctx.struct_vars.borrow_mut() = saved_struct_vars;
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::ForLoop { pattern, iter, body, .. } => {
            out.push_str("for ");
            match pattern {
                ForPattern::Single(n) => out.push_str(n),
                ForPattern::Tuple(ns) => { out.push('('); out.push_str(&ns.join(", ")); out.push(')'); }
            }
            out.push_str(" in "); gen_expr(iter, out, ctx); out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx, result_fn, declared);
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::If { condition, body, elifs, else_body, .. } => {
            out.push_str("if "); gen_expr(condition, out, ctx); out.push_str(" {\n");
            let mut branch_decl = declared.clone();
            gen_stmts(body, out, indent + 1, ctx, result_fn, &mut branch_decl);
            out.push_str(&pad); out.push('}');
            for (cond, block) in elifs {
                out.push_str(" else if "); gen_expr(cond, out, ctx); out.push_str(" {\n");
                let mut elif_decl = declared.clone();
                gen_stmts(block, out, indent + 1, ctx, result_fn, &mut elif_decl);
                out.push_str(&pad); out.push('}');
            }
            if let Some(eb) = else_body {
                out.push_str(" else {\n");
                let mut else_decl = declared.clone();
                gen_stmts(eb, out, indent + 1, ctx, result_fn, &mut else_decl);
                out.push_str(&pad); out.push('}');
            }
            out.push('\n');
        }
        Stmt::Match { expr, arms, .. } => {
            let has_string_pat = arms.iter().any(|a| matches!(&a.pattern, MatchPattern::StringLit(_)));
            let has_list_pat = arms.iter().any(|a| matches!(&a.pattern, MatchPattern::List(_)));

            if has_list_pat {
                // Generate if-else chain for list patterns
                let temp_var = format!("_match_list_{}", out.len());
                out.push_str("let "); out.push_str(&temp_var); out.push_str(" = "); gen_expr(expr, out, ctx); out.push_str(";\n");

                for (i, arm) in arms.iter().enumerate() {
                    out.push_str(&pad);
                    let is_first = i == 0;
                    let prefix = if is_first { "if" } else { " else if" };

                    match &arm.pattern {
                        MatchPattern::List(ListPattern::Empty) => {
                            out.push_str(prefix); out.push_str(" "); out.push_str(&temp_var); out.push_str(".is_empty()");
                            out.push_str(" {\n");
                        }
                        MatchPattern::List(ListPattern::Single(x)) => {
                            out.push_str(prefix); out.push_str(" "); out.push_str(&temp_var); out.push_str(".len() == 1");
                            out.push_str(" {\n");
                            out.push_str(&pad); out.push_str("    ");
                            out.push_str("let "); out.push_str(x); out.push_str(" = "); out.push_str(&temp_var); out.push_str("[0];\n");
                        }
                        MatchPattern::List(ListPattern::Cons(head, tail)) => {
                            out.push_str(prefix); out.push_str(" !"); out.push_str(&temp_var); out.push_str(".is_empty()");
                            out.push_str(" {\n");
                            out.push_str(&pad); out.push_str("    ");
                            out.push_str("let "); out.push_str(head); out.push_str(" = "); out.push_str(&temp_var); out.push_str("[0];\n");
                            out.push_str(&pad); out.push_str("    ");
                            out.push_str("let "); out.push_str(tail); out.push_str(" = &"); out.push_str(&temp_var); out.push_str("[1..];\n");
                        }
                        MatchPattern::Wildcard => {
                            out.push_str(" else"); out.push_str(" {\n");
                        }
                        _ => {
                            out.push_str(prefix); out.push_str(" true {\n");
                        }
                    }
                    gen_stmt(&arm.body, out, indent + 1, ctx, result_fn, declared);
                    out.push_str(&pad); out.push_str("}");
                }
                out.push('\n');
            } else {
                out.push_str("match "); gen_expr(expr, out, ctx);
                if has_string_pat { out.push_str(".as_str()"); }
                out.push_str(" {\n");
                for arm in arms {
                    out.push_str(&pad); out.push_str("    ");
                    match &arm.pattern {
                        MatchPattern::Variant { name, bindings } => {
                            // Handle built-in Option/Result variants
                            if name == "None" {
                                out.push_str("None");
                            } else if name == "Some" || name == "Ok" || name == "Err" {
                                out.push_str(name); out.push('('); out.push_str(&bindings.join(", ")); out.push(')');
                            } else if let Some(en) = ctx.variant_to_enum.get(name) {
                                out.push_str(en); out.push_str("::");
                                out.push_str(name);
                                if !bindings.is_empty() {
                                    out.push('('); out.push_str(&bindings.join(", ")); out.push(')');
                                }
                            } else {
                                out.push_str(name);
                                if !bindings.is_empty() {
                                    out.push('('); out.push_str(&bindings.join(", ")); out.push(')');
                                }
                            }
                        }
                        MatchPattern::StringLit(s) => { out.push('"'); out.push_str(s); out.push('"'); }
                        MatchPattern::Wildcard => { out.push('_'); }
                        _ => {}
                    }
                    out.push_str(" => {\n");
                    gen_stmt(&arm.body, out, indent + 2, ctx, result_fn, declared);
                    out.push_str(&pad); out.push_str("    }\n");
                }
                out.push_str(&pad); out.push_str("}\n");
            }
        }
        Stmt::Return { value, .. } => {
            if result_fn {
                if let Some(val) = value {
                    out.push_str("return Ok("); gen_expr(val, out, ctx);
                    if is_plain_string(val) { out.push_str(".to_string()"); }
                    out.push_str(");\n");
                } else {
                    out.push_str("return Ok(());\n");
                }
            } else {
                out.push_str("return");
                if let Some(val) = value {
                    out.push(' '); gen_expr(val, out, ctx);
                    if is_plain_string(val) { out.push_str(".to_string()"); }
                }
                out.push_str(";\n");
            }
        }
        Stmt::MutAssign { object, field, value, .. } => {
            gen_expr(object, out, ctx); out.push('.'); out.push_str(field); out.push_str(" = ");
            gen_expr(value, out, ctx);
            if is_plain_string(value) { out.push_str(".to_string()"); }
            out.push_str(";\n");
        }
        Stmt::Loop { body, .. } => {
            out.push_str("loop {\n");
            gen_stmts(body, out, indent + 1, ctx, result_fn, declared);
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::WhileLoop { condition, body, .. } => {
            out.push_str("while "); gen_expr(condition, out, ctx); out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx, result_fn, declared);
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::Break { .. } => {
            out.push_str("break;\n");
        }
        Stmt::Continue { .. } => {
            out.push_str("continue;\n");
        }
        Stmt::Spawn { expr, .. } => {
            // Unwrap lambda — spawn fn() expr is equivalent to spawn expr
            let spawn_body = match expr {
                Expr::Lambda { body, .. } => body.as_ref(),
                _ => expr,
            };
            // Clone captured variables so move closure works
            let mut vars = collect_free_vars(spawn_body);
            vars.sort(); vars.dedup();
            vars.retain(|v| !ctx.fn_params.contains_key(v));
            out.push_str("{\n");
            for v in &vars {
                out.push_str(&pad); out.push_str("    let ");
                out.push_str(v); out.push_str(" = ");
                // Для каналов клонируем только Sender (первый элемент кортежа)
                if v.starts_with("ch") {
                    out.push_str(v); out.push_str(".0.clone();\n");
                } else {
                    out.push_str(v); out.push_str(".clone();\n");
                }
            }
            out.push_str(&pad); out.push_str("    tokio::spawn(async move {\n");
            out.push_str(&pad); out.push_str("        ");
            gen_expr(spawn_body, out, ctx);
            out.push_str(";\n");
            out.push_str(&pad); out.push_str("    });\n");
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::TraitDef { name, methods, .. } => {
            out.push_str("trait "); out.push_str(name); out.push_str(" {\n");
            for sig in methods {
                out.push_str(&pad); out.push_str("    fn "); out.push_str(&sig.name); out.push('(');
                for (i, p) in sig.params.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    if p.name == "self" {
                        out.push_str(if p.is_mut { "&mut self" } else { "&self" });
                    } else {
                        out.push_str(&p.name); out.push_str(": ");
                        if let Some(ref t) = p.type_ann { out.push_str(&map_param_type(t, &ctx.structs, p.is_mut)); }
                        else { out.push_str("i64"); }
                    }
                }
                out.push(')');
                if let Some(ref ret) = sig.return_type {
                    let rt = map_return_type(ret, &ctx.structs);
                    out.push_str(" -> "); out.push_str(&rt);
                }
                out.push_str(";\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::ImplBlock { trait_name, target, methods, .. } => {
            out.push_str("impl ");
            if let Some(tn) = trait_name { out.push_str(tn); out.push_str(" for "); }
            out.push_str(target); out.push_str(" {\n");
            for method in methods {
                if let Stmt::FnDef { name, params, return_type, body, span: mspan, .. } = method {
                    let fn_uses_q = uses_qmark(body);
                    let has_ret = has_return_value(body);
                    let mpad = "    ".repeat(indent + 1);
                    // Line comment for method
                    if !ctx.filename.is_empty() {
                        let mline = ctx.line_of(mspan.start);
                        out.push_str(&format!("{}// line:{}\n", mpad, mline));
                    }
                    out.push_str(&mpad);
                    if ctx.async_fns.contains(name.as_str()) { out.push_str("async "); }
                    out.push_str("fn "); out.push_str(name); out.push('(');
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 { out.push_str(", "); }
                        if p.name == "self" {
                            out.push_str(if p.is_mut { "&mut self" } else { "&self" });
                        } else {
                            out.push_str(&p.name); out.push_str(": ");
                            if let Some(ref t) = p.type_ann { out.push_str(&map_param_type(t, &ctx.structs, p.is_mut)); }
                            else { out.push_str(&infer_param_type(&p.name, body, ctx, return_type.as_deref(), p.is_mut)); }
                        }
                    }
                    out.push(')');
                    if fn_uses_q {
                        let ret = return_type.as_deref()
                            .map(|t| map_return_type(t, &ctx.structs))
                            .unwrap_or_else(|| if has_ret { infer_method_ret(body, target, &ctx.structs) } else { "()".to_string() });
                        out.push_str(" -> Result<"); out.push_str(&ret);
                        out.push_str(", Box<dyn std::error::Error + Send + Sync>>");
                    } else if has_ret {
                        let ret = return_type.as_deref()
                            .map(|t| map_return_type(t, &ctx.structs))
                            .unwrap_or_else(|| infer_method_ret(body, target, &ctx.structs));
                        out.push_str(" -> "); out.push_str(&ret);
                    }
                    out.push_str(" {\n");
                    let mut fn_declared = HashSet::new();
                    gen_stmts(body, out, indent + 2, ctx, fn_uses_q, &mut fn_declared);
                    if fn_uses_q && !has_ret { out.push_str(&mpad); out.push_str("    Ok(())\n"); }
                    out.push_str(&mpad); out.push_str("}\n");
                }
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::MemoryDecl { .. } | Stmt::UseDecl { .. } => { /* unreachable — filtered above */ }
    }
}

fn gen_expr(expr: &Expr, out: &mut String, ctx: &Ctx) {
    match expr {
        Expr::StringLiteral { parts, .. } => gen_string(parts, out, ctx),
        Expr::IntLiteral { value, .. } => { out.push_str(&value.to_string()); out.push_str("_i64"); }
        Expr::FloatLiteral { value, .. } => {
            let s = value.to_string(); out.push_str(&s);
            if !s.contains('.') { out.push_str(".0"); }
            out.push_str("_f64");
        }
        Expr::BoolLiteral { value, .. } => out.push_str(if *value { "true" } else { "false" }),
        Expr::NoneLiteral { .. } => out.push_str("None"),
        Expr::Identifier { name, .. } => {
            if let Some(en) = ctx.variant_to_enum.get(name) {
                out.push_str(en); out.push_str("::"); out.push_str(name);
            } else {
                out.push_str(name);
            }
        }
        Expr::FunctionCall { name, args, .. } => {
            if name == "print" { gen_print(args, out, ctx); return; }
            // Channel.new() → tokio::sync::mpsc::channel(100)
            if name == "Channel.new" {
                out.push_str("{ let (tx, rx) = tokio::sync::mpsc::channel(100); (tx, rx) }");
                return;
            }
            // range(start, end) → range2(start, end)
            let fn_name = if name == "range" && args.len() == 2 { "range2" } else { name.as_str() };
            out.push_str(fn_name); out.push('(');
            // Check if we have fn param info for adding & where needed
            if let Some(fn_p) = ctx.fn_params.get(name.as_str()) {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    if is_struct_var_ident(arg, ctx) {
                        let param_mut = fn_p.get(i).map(|p| p.is_mut).unwrap_or(false);
                        if param_mut { out.push_str("&mut "); } else { out.push_str("&"); }
                        gen_expr(arg, out, ctx);
                    } else {
                        let type_ann = fn_p.get(i).and_then(|p| p.type_ann.as_deref());
                        let needs_ref = type_ann.map(|t| is_ref_type(t) && !ctx.structs.contains(t)).unwrap_or(false);
                        if needs_ref { out.push('&'); }
                        gen_expr(arg, out, ctx);
                    }
                }
            } else if let Some(param_types) = runtime_param_types(name) {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    if param_types.get(i).map(|t| t.starts_with('&')).unwrap_or(false) {
                        out.push('&');
                    }
                    gen_expr(arg, out, ctx);
                }
            } else {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    gen_expr(arg, out, ctx);
                }
            }
            out.push(')');
            // .await for async functions (user-defined async + runtime async)
            if ctx.async_fns.contains(name.as_str()) || is_async_function(name) {
                out.push_str(".await");
            }
        }
        Expr::Lambda { params, body, .. } => {
            out.push('|'); out.push_str(&params.join(", ")); out.push_str("| ");
            gen_expr(body, out, ctx);
        }
        Expr::MethodCall { object, method, args, .. } => {
            // Static method call: StructName.method() → StructName::method()
            if let Expr::Identifier { name, .. } = object.as_ref() {
                if ctx.structs.contains(name.as_str()) {
                    out.push_str(name); out.push_str("::"); out.push_str(method);
                    out.push('('); gen_args(args, out, ctx); out.push(')');
                    if is_async_method(method) { out.push_str(".await"); }
                    return;
                }
            }
            // Dot methods that map to runtime free functions: obj.find(x) → u_runtime::find(&obj, &x)
            let rt_method = match method.as_str() {
                "find" if args.len() == 1 => Some(("find", &["&str", "&str"] as &[&str])),
                "find_from" => Some(("find_from", &["&str", "&str", "i64"] as &[&str])),
                "slice" => Some(("slice_range", &["&str", "i64", "i64"] as &[&str])),
                "slice_from" => Some(("slice_from", &["&str", "i64"] as &[&str])),
                "split_lines" if args.is_empty() => Some(("split_lines", &["&str"] as &[&str])),
                "split" if args.len() == 1 => Some(("split", &["&str", "&str"] as &[&str])),
                _ => None,
            };
            if let Some((rt_fn, param_types)) = rt_method {
                out.push_str("u_runtime::"); out.push_str(rt_fn); out.push('(');
                // First param is the object
                if param_types.first() == Some(&"&str") { out.push('&'); }
                gen_expr(object, out, ctx);
                for (i, arg) in args.iter().enumerate() {
                    out.push_str(", ");
                    if param_types.get(i + 1) == Some(&"&str") { out.push('&'); }
                    gen_expr(arg, out, ctx);
                }
                out.push(')');
                return;
            }
            // .len() → .len() as i64 (Rust returns usize, U uses i64)
            if method == "len" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".len() as i64");
                return;
            }
            // .first() → .first().copied().unwrap_or(0)
            if method == "first" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".first().copied().unwrap_or(0)");
                return;
            }
            // .last() → .last().copied().unwrap_or(0)
            if method == "last" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".last().copied().unwrap_or(0)");
                return;
            }
            // .to_int() → str_to_int(&s) возвращает Option<i64>
            if method == "to_int" && args.is_empty() {
                out.push_str("u_runtime::str_to_int(&");
                gen_expr(object, out, ctx);
                out.push_str(")");
                return;
            }
            // .to_float() → str_to_float(&s) возвращает Option<f64>
            if method == "to_float" && args.is_empty() {
                out.push_str("u_runtime::str_to_float(&");
                gen_expr(object, out, ctx);
                out.push_str(")");
                return;
            }
            // .sum() → .iter().sum::<i64>()
            if method == "sum" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".iter().sum::<i64>()");
                return;
            }
            // .sort() → .clone(); v.sort(); v
            if method == "sort" && args.is_empty() {
                out.push_str("{ let mut __v = ");
                gen_expr(object, out, ctx);
                out.push_str(".clone(); __v.sort(); __v }");
                return;
            }
            // .reverse() → .clone().into_iter().rev().collect::<Vec<_>>()
            if method == "reverse" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".clone().into_iter().rev().collect::<Vec<_>>()");
                return;
            }
            // .is_empty() → .is_empty()
            if method == "is_empty" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".is_empty()");
                return;
            }
            // Channel methods: .send() and .receive()
            // Channel is (Sender<T>, Receiver<T>) tuple
            // .send(data) → tx.send(data).await.unwrap()
            if method == "send" && args.len() == 1 {
                out.push_str("{ let (tx, _) = ");
                gen_expr(object, out, ctx);
                out.push_str("; tx.send(");
                gen_expr(&args[0], out, ctx);
                out.push_str(").await.unwrap() }");
                return;
            }
            // .receive() → rx.recv().await.unwrap()
            if method == "receive" && args.is_empty() {
                out.push_str("{ let (_, rx) = ");
                gen_expr(object, out, ctx);
                out.push_str("; rx.recv().await.unwrap() }");
                return;
            }
            // .append(item) → .clone().push(item) (returns new vec)
            if method == "append" && args.len() == 1 {
                out.push_str("{ let mut __v = ");
                gen_expr(object, out, ctx);
                out.push_str(".clone(); __v.push(");
                gen_expr(&args[0], out, ctx);
                out.push_str("); __v }");
                return;
            }
            // .to_upper() → .to_uppercase()
            if method == "to_upper" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".to_uppercase()");
                return;
            }
            // .to_lower() → .to_lowercase()
            if method == "to_lower" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".to_lowercase()");
                return;
            }
            // .to_string() для Int/Float
            if method == "to_string" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".to_string()");
                return;
            }
            // .is_ok() → .is_ok()
            if method == "is_ok" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".is_ok()");
                return;
            }
            // .is_err() → .is_err()
            if method == "is_err" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".is_err()");
                return;
            }
            // .unwrap() → .unwrap()
            if method == "unwrap" && args.is_empty() {
                gen_expr(object, out, ctx);
                out.push_str(".unwrap()");
                return;
            }
            // .unwrap_or(default) → .unwrap_or(default)
            if method == "unwrap_or" && args.len() == 1 {
                gen_expr(object, out, ctx);
                out.push_str(".unwrap_or(");
                gen_expr(&args[0], out, ctx);
                out.push_str(")");
                return;
            }
            // .recv_timeout(ms) → .recv_timeout(ms).await
            if method == "recv_timeout" && args.len() == 1 {
                gen_expr(object, out, ctx);
                out.push_str(".recv_timeout(");
                gen_expr(&args[0], out, ctx);
                out.push_str(").await");
                return;
            }
            // .filter(fn(x) expr) → .into_iter().filter(|x| expr).collect::<Vec<_>>()
            // clone() preserves original, into_iter() gives owned values, filter gets &i64
            if method == "filter" && args.len() == 1 {
                if let Expr::Lambda { params, body, .. } = &args[0] {
                    gen_expr(object, out, ctx);
                    out.push_str(".clone().into_iter().filter(|&");
                    out.push_str(&params.join(", &"));
                    out.push_str("| ");
                    gen_expr(body, out, ctx);
                    out.push_str(").collect::<Vec<_>>()");
                    return;
                }
            }
            // .map(fn(x) expr) → .into_iter().map(|x| expr).collect::<Vec<_>>()
            if method == "map" && args.len() == 1 {
                if let Expr::Lambda { params, body, .. } = &args[0] {
                    gen_expr(object, out, ctx);
                    out.push_str(".clone().into_iter().map(|");
                    out.push_str(&params.join(", "));
                    out.push_str("| ");
                    gen_expr(body, out, ctx);
                    out.push_str(").collect::<Vec<_>>()");
                    return;
                }
            }
            // Native string methods needing &str args
            if matches!(method.as_str(), "replace" | "starts_with" | "ends_with" | "contains") {
                gen_expr(object, out, ctx);
                out.push('.'); out.push_str(method); out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push('&');
                    gen_expr(arg, out, ctx);
                }
                out.push(')');
                return;
            }
            gen_expr(object, out, ctx);
            // exec/query with 2+ args → exec1/query1 with &second_arg
            if (method == "exec" || method == "query") && args.len() > 1 {
                out.push('.'); out.push_str(method); out.push_str("1(");
                gen_expr(&args[0], out, ctx); // SQL string
                for arg in &args[1..] {
                    out.push_str(", &"); gen_expr(arg, out, ctx);
                }
                out.push(')');
            } else {
                out.push('.'); out.push_str(method); out.push('('); gen_args(args, out, ctx); out.push(')');
            }
            // .await for async methods
            if is_async_method(method) {
                out.push_str(".await");
            }
        }
        Expr::FieldAccess { object, field, .. } => {
            gen_expr(object, out, ctx); out.push('.'); out.push_str(field);
        }
        Expr::PostfixOp { expr, op, .. } => {
            gen_expr(expr, out, ctx);
            if op == "!" { out.push_str(".unwrap()"); } else { out.push_str(op); }
        }
        Expr::BinaryOp { left, op, right, .. } => {
            gen_expr(left, out, ctx); out.push(' '); out.push_str(op); out.push(' ');
            gen_expr(right, out, ctx);
        }
        Expr::UnaryOp { op, expr, .. } => {
            if op == "not" { out.push('!'); } else { out.push_str(op); }
            gen_expr(expr, out, ctx);
        }
        Expr::Index { object, index, .. } => {
            // Handle Phantom[T] syntax
            if let Expr::Identifier { name: obj_name, .. } = object.as_ref() {
                if obj_name == "Phantom" {
                    out.push_str("std::marker::PhantomData");
                    if let Expr::Identifier { name: type_name, .. } = index.as_ref() {
                        out.push_str("::<");
                        out.push_str(&map_type(type_name));
                        out.push_str(">");
                    }
                    return;
                }
            }
            gen_expr(object, out, ctx);
            out.push_str(".get(");
            gen_expr(index, out, ctx);
            out.push_str(" as usize).cloned().unwrap()");
        }
        Expr::List { elements, .. } => {
            out.push_str("vec!["); gen_args(elements, out, ctx); out.push(']');
        }
        Expr::StructInit { name, fields, .. } => {
            // Handle Phantom[T] -> std::marker::PhantomData
            if name == "Phantom" {
                out.push_str("std::marker::PhantomData");
                return;
            }
            if ctx.structs.contains(name) {
                out.push_str(name); out.push_str(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(fname); out.push_str(": "); gen_expr(fval, out, ctx);
                    if is_plain_string(fval) { out.push_str(".to_string()"); }
                }
                out.push_str(" }");
            } else if let Some(en) = ctx.variant_to_enum.get(name) {
                out.push_str(en); out.push_str("::"); out.push_str(name);
                if !fields.is_empty() {
                    out.push('(');
                    for (i, (_, fval)) in fields.iter().enumerate() {
                        if i > 0 { out.push_str(", "); }
                        gen_expr(fval, out, ctx);
                    }
                    out.push(')');
                }
            } else {
                out.push_str(name); out.push('('); gen_args(&fields.iter().map(|(_, v)| v).cloned().collect::<Vec<_>>(), out, ctx); out.push(')');
            }
        }
    }
}

fn gen_args(args: &[Expr], out: &mut String, ctx: &Ctx) {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 { out.push_str(", "); }
        gen_expr(arg, out, ctx);
    }
}

fn raw_delimiters(parts: &[StringPart]) -> (String, String) {
    let has_quote = parts.iter().any(|p| matches!(p, StringPart::Text(t) if t.contains('"')));
    if !has_quote { return ("\"".into(), "\"".into()); }
    let all_text: String = parts.iter()
        .filter_map(|p| if let StringPart::Text(t) = p { Some(t.as_str()) } else { None })
        .collect();
    let mut n = 1;
    loop {
        let end = format!("\"{}", "#".repeat(n));
        if !all_text.contains(&end) {
            return (format!("r{}\"", "#".repeat(n)), end);
        }
        n += 1;
    }
}

fn gen_string(parts: &[StringPart], out: &mut String, ctx: &Ctx) {
    let has_interp = parts.iter().any(|p| matches!(p, StringPart::Interpolation(_)));
    let (open, close) = raw_delimiters(parts);
    if has_interp {
        let fmt = build_format(parts, ctx);
        out.push_str("format!("); out.push_str(&open); out.push_str(&fmt.template); out.push_str(&close);
        for arg in &fmt.args { out.push_str(", "); out.push_str(arg); }
        out.push(')');
    } else {
        out.push_str(&open);
        for p in parts { if let StringPart::Text(t) = p { out.push_str(t); } }
        out.push_str(&close);
    }
}

fn gen_print(args: &[Expr], out: &mut String, ctx: &Ctx) {
    if args.len() == 1 {
        if let Expr::StringLiteral { parts, .. } = &args[0] {
            let fmt = build_format(parts, ctx);
            let (open, close) = raw_delimiters(parts);
            out.push_str("println!("); out.push_str(&open); out.push_str(&fmt.template); out.push_str(&close);
            for arg in &fmt.args { out.push_str(", "); out.push_str(arg); }
            out.push(')');
        } else {
            out.push_str("println!(\"{}\", "); gen_expr(&args[0], out, ctx); out.push(')');
        }
    } else if args.is_empty() { out.push_str("println!()"); }
    else {
        out.push_str("println!(\"");
        for i in 0..args.len() { if i > 0 { out.push(' '); } out.push_str("{}"); }
        out.push_str("\", "); gen_args(args, out, ctx); out.push(')');
    }
}

struct FormatParts { template: String, args: Vec<String> }
fn build_format(parts: &[StringPart], ctx: &Ctx) -> FormatParts {
    let mut template = String::new();
    let mut args = Vec::new();
    for part in parts {
        match part {
            StringPart::Text(text) => {
                for c in text.chars() {
                    match c { '{' => template.push_str("{{"), '}' => template.push_str("}}"), _ => template.push(c) }
                }
            }
            StringPart::Interpolation(expr) => {
                template.push_str("{}");
                let mut arg = String::new(); gen_expr(expr, &mut arg, ctx); args.push(arg);
            }
        }
    }
    FormatParts { template, args }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn gen(src: &str) -> Result<String, String> {
        generate(&parser::parse(src).unwrap(), src, "test.u", &[], &HashMap::new())
    }

    #[test]
    fn test_hello() {
        let code = gen("name = \"Мир\"\nprint(\"Привет, $name!\")").unwrap();
        assert!(code.contains("println!(\"Привет, {}!\", name)"));
    }

    #[test]
    fn test_fn_result() {
        let code = gen("fn f() -> Db\n    x = Sqlite.open(\"a\")?\n    return x\nend").unwrap();
        assert!(code.contains("-> Result<Db, Box<dyn std::error::Error + Send + Sync>>"));
        assert!(code.contains("return Ok(x)"));
    }

    #[test]
    fn test_fn_ref_params() {
        let code = gen("fn f(db: Db, t: String)\n    print(t)\nend\nf(db, title)").unwrap();
        assert!(code.contains("fn f(db: &Db, t: &str)"));
        assert!(code.contains("f(&db, &title)"));
    }

    #[test]
    fn test_exec1() {
        let code = gen("x = db.exec(\"sql\", val)").unwrap();
        assert!(code.contains("db.exec1(\"sql\", &val)"));
    }

    #[test]
    fn test_match_string() {
        let code = gen("match cmd\n    \"a\" => print(\"ok\")\n    _ => print(\"no\")\nend").unwrap();
        assert!(code.contains("match cmd.as_str()"));
        assert!(code.contains("\"a\" =>"));
        assert!(code.contains("_ =>"));
    }

    #[test]
    fn test_spawn_mut_param_rejected() {
        let src = "fn writer(mut data)\n    print(data)\nend\nspawn writer(x)";
        let result = generate(&parser::parse(src).unwrap(), src, "test.u", &[], &HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot mutate external variable in spawn"));
    }

    #[test]
    fn test_impl_method() {
        let src = "struct Foo\n    x: Int\nend\n\nimpl Foo\n    fn get(self) -> Int\n        return self.x\n    end\nend\n\nf = Foo(x: 1)\nprint(f.get())";
        let code = gen(src).unwrap();
        assert!(code.contains("impl Foo {"));
        assert!(code.contains("fn get(&self) -> i64"));
    }

    #[test]
    fn test_impl_static_method() {
        let src = "struct Bar\n    v: Int\nend\n\nimpl Bar\n    fn new()\n        return Bar(v: 0)\n    end\nend\n\nb = Bar.new()";
        let code = gen(src).unwrap();
        assert!(code.contains("fn new() -> Bar"));
        assert!(code.contains("Bar::new()"));
    }

    #[test]
    fn test_trait_def() {
        let src = "trait Show\n    fn show(self) -> String\nend";
        let code = gen(src).unwrap();
        assert!(code.contains("trait Show {"));
        assert!(code.contains("fn show(&self) -> String;"));
    }

    #[test]
    fn test_struct_stack_alloc() {
        let code = gen("struct User\n    name: String\n    age: Int\nend\n\nu = User(name: \"test\", age: 1)\nprint(u.name)").unwrap();
        assert!(code.contains("User { name:"));
        assert!(!code.contains("Rc::new"));
        assert!(code.contains("u.name"));
        assert!(!code.contains("borrow()"));
    }

    #[test]
    fn test_struct_mut_assign() {
        let code = gen("struct P\n    x: Int\nend\n\np = P(x: 1)\np.x = 2").unwrap();
        assert!(code.contains("p.x = 2"));
        assert!(!code.contains("borrow_mut()"));
    }

    #[test]
    fn test_struct_fn_param_infer() {
        let code = gen("struct User\n    name: String\nend\n\nfn show(u)\n    print(u.name)\nend").unwrap();
        assert!(code.contains("fn show(u: &User)"));
        assert!(code.contains("u.name"));
    }

    #[test]
    fn test_struct_fn_call_ref() {
        let code = gen("struct P\n    x: Int\nend\n\nfn f(p)\n    print(p.x)\nend\n\np = P(x: 1)\nf(p)\nf(p)").unwrap();
        assert!(code.contains("f(&p)"));
        assert!(!code.contains("clone()"));
    }
}
