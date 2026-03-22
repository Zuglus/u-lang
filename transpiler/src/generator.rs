use std::collections::{HashMap, HashSet};
use crate::ast::*;

struct Ctx {
    structs: HashSet<String>,
    variant_to_enum: HashMap<String, String>,
    fn_params: HashMap<String, Vec<FnParam>>,
    async_fns: HashSet<String>,
}

impl Ctx {
    fn from_program(program: &Program) -> Self {
        let mut ctx = Ctx { structs: HashSet::new(), variant_to_enum: HashMap::new(), fn_params: HashMap::new(), async_fns: HashSet::new() };
        for stmt in &program.statements {
            match stmt {
                Stmt::StructDef { name, .. } => { ctx.structs.insert(name.clone()); }
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
        ctx
    }
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
        Stmt::Loop { body, .. } => stmts_need_async(body),
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
        Stmt::Loop { body, .. } => stmts_call_any_async(body, af),
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

fn map_type(t: &str) -> &str {
    match t { "Int" => "i64", "Float" => "f64", "String" => "String", "Bool" => "bool", "Channel" => "Chan", o => o }
}

fn map_param_type(t: &str) -> String {
    match t {
        "Int" => "i64".into(), "Float" => "f64".into(), "Bool" => "bool".into(),
        "String" => "&str".into(), "Db" => "&Db".into(),
        "Channel" => "Chan".into(),
        other => format!("&{}", other),
    }
}

fn is_ref_type(t: &str) -> bool {
    !matches!(t, "Int" | "Float" | "Bool" | "Channel")
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
        Stmt::ForLoop { body, .. } | Stmt::Loop { body, .. } => has_return_value(body),
        Stmt::Match { arms, .. } => arms.iter().any(|a| matches!(&a.body, Stmt::Return { value: Some(_), .. })),
        _ => false,
    })
}

fn infer_param_type(param: &str, body: &[Stmt], ctx: &Ctx) -> String {
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
    "i64".to_string()
}

pub fn generate(program: &Program) -> Result<String, String> {
    let ctx = Ctx::from_program(program);
    validate_spawn_safety(program, &ctx)?;

    let mut out = String::new();
    out.push_str("#![allow(unused_mut, unused_variables, dead_code, unused_imports)]\n");
    out.push_str("use u_runtime::*;\n\n");

    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. } => {
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
        if !matches!(stmt, Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. } | Stmt::MemoryDecl { .. } | Stmt::UseDecl { .. }) {
            gen_stmt(stmt, &mut out, 1, &ctx, true, &mut main_declared);
        }
    }
    out.push_str("    Ok(())\n}\n");
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
                            "error: cannot mutate external data from goroutine — '::' changes bytes in shared memory. Use a channel (.send) instead.\n  spawn {}(...) ← parameter '::{}' mutates shared data",
                            name, p.name
                        ));
                    }
                }
            }
        }
        Stmt::FnDef { body, .. } | Stmt::ForLoop { body, .. } | Stmt::Loop { body, .. } => {
            for s in body { validate_stmt_spawn(s, ctx)?; }
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
    matches!(name, "sleep")
}

fn is_async_method(method: &str) -> bool {
    matches!(method, "recv" | "accept" | "listen" | "respond" | "path")
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
    out.push_str(&pad);
    match stmt {
        Stmt::StructDef { name, fields, .. } => {
            out.push_str("#[derive(Debug, Clone)]\nstruct ");
            out.push_str(name);
            out.push_str(" {\n");
            for f in fields {
                out.push_str(&pad); out.push_str("    ");
                out.push_str(&f.name); out.push_str(": "); out.push_str(map_type(&f.type_name)); out.push_str(",\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::TypeDef { name, variants, .. } => {
            out.push_str("#[derive(Debug, Clone)]\nenum ");
            out.push_str(name); out.push_str(" {\n");
            for v in variants {
                out.push_str(&pad); out.push_str("    "); out.push_str(&v.name); out.push('(');
                for (i, f) in v.fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(map_type(&f.type_name));
                }
                out.push_str("),\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::Assignment { name, value, .. } => {
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
        Stmt::FnDef { name, params, return_type, body, .. } => {
            let fn_uses_q = uses_qmark(body);
            let has_ret = has_return_value(body);
            if ctx.async_fns.contains(name) { out.push_str("async "); }
            out.push_str("fn "); out.push_str(name); out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(&p.name); out.push_str(": ");
                if let Some(ref t) = p.type_ann {
                    out.push_str(&map_param_type(t));
                } else {
                    out.push_str(&infer_param_type(&p.name, body, ctx));
                }
            }
            out.push(')');
            if fn_uses_q {
                let ret = return_type.as_deref().map(map_type).unwrap_or(if has_ret { "i64" } else { "()" });
                out.push_str(" -> Result<"); out.push_str(ret); out.push_str(", Box<dyn std::error::Error + Send + Sync>>");
            } else if has_ret {
                let ret = return_type.as_deref().map(map_type).unwrap_or("i64");
                out.push_str(" -> "); out.push_str(ret);
            }
            out.push_str(" {\n");
            let mut fn_declared = HashSet::new();
            gen_stmts(body, out, indent + 1, ctx, fn_uses_q, &mut fn_declared);
            if fn_uses_q && !has_ret { out.push_str(&pad); out.push_str("    Ok(())\n"); }
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
            gen_stmts(body, out, indent + 1, ctx, result_fn, declared);
            out.push_str(&pad); out.push('}');
            for (cond, block) in elifs {
                out.push_str(" else if "); gen_expr(cond, out, ctx); out.push_str(" {\n");
                gen_stmts(block, out, indent + 1, ctx, result_fn, declared);
                out.push_str(&pad); out.push('}');
            }
            if let Some(eb) = else_body {
                out.push_str(" else {\n"); gen_stmts(eb, out, indent + 1, ctx, result_fn, declared);
                out.push_str(&pad); out.push('}');
            }
            out.push('\n');
        }
        Stmt::Match { expr, arms, .. } => {
            let has_string_pat = arms.iter().any(|a| matches!(&a.pattern, MatchPattern::StringLit(_)));
            out.push_str("match "); gen_expr(expr, out, ctx);
            if has_string_pat { out.push_str(".as_str()"); }
            out.push_str(" {\n");
            for arm in arms {
                out.push_str(&pad); out.push_str("    ");
                match &arm.pattern {
                    MatchPattern::Variant { name, bindings } => {
                        if let Some(en) = ctx.variant_to_enum.get(name) { out.push_str(en); out.push_str("::"); }
                        out.push_str(name); out.push('('); out.push_str(&bindings.join(", ")); out.push_str(")");
                    }
                    MatchPattern::StringLit(s) => { out.push('"'); out.push_str(s); out.push('"'); }
                    MatchPattern::Wildcard => { out.push('_'); }
                }
                out.push_str(" => {\n");
                gen_stmt(&arm.body, out, indent + 2, ctx, result_fn, declared);
                out.push_str(&pad); out.push_str("    }\n");
            }
            out.push_str(&pad); out.push_str("}\n");
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
            gen_expr(value, out, ctx); out.push_str(";\n");
        }
        Stmt::Loop { body, .. } => {
            out.push_str("loop {\n");
            gen_stmts(body, out, indent + 1, ctx, result_fn, declared);
            out.push_str(&pad); out.push_str("}\n");
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
                out.push_str(v); out.push_str(" = "); out.push_str(v); out.push_str(".clone();\n");
            }
            out.push_str(&pad); out.push_str("    tokio::spawn(async move {\n");
            out.push_str(&pad); out.push_str("        ");
            gen_expr(spawn_body, out, ctx);
            out.push_str(";\n");
            out.push_str(&pad); out.push_str("    });\n");
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
        Expr::Identifier { name, .. } => out.push_str(name),
        Expr::FunctionCall { name, args, .. } => {
            if name == "print" { gen_print(args, out, ctx); return; }
            out.push_str(name); out.push('(');
            // Check if we have fn param info for adding & where needed
            if let Some(fn_p) = ctx.fn_params.get(name.as_str()) {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    let needs_ref = fn_p.get(i).and_then(|p| p.type_ann.as_deref()).map(is_ref_type).unwrap_or(false);
                    if needs_ref { out.push('&'); }
                    gen_expr(arg, out, ctx);
                }
            } else if let Some(param_types) = runtime_param_types(name) {
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    if param_types.get(i) == Some(&"&str") {
                        out.push('&');
                    }
                    gen_expr(arg, out, ctx);
                }
            } else {
                gen_args(args, out, ctx);
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
        Expr::List { elements, .. } => {
            out.push_str("vec!["); gen_args(elements, out, ctx); out.push(']');
        }
        Expr::StructInit { name, fields, .. } => {
            if ctx.structs.contains(name) {
                out.push_str(name); out.push_str(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(fname); out.push_str(": "); gen_expr(fval, out, ctx);
                }
                out.push_str(" }");
            } else if let Some(en) = ctx.variant_to_enum.get(name) {
                out.push_str(en); out.push_str("::"); out.push_str(name); out.push('(');
                for (i, (_, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    gen_expr(fval, out, ctx);
                }
                out.push(')');
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

fn gen_string(parts: &[StringPart], out: &mut String, ctx: &Ctx) {
    let has_interp = parts.iter().any(|p| matches!(p, StringPart::Interpolation(_)));
    if has_interp {
        let fmt = build_format(parts, ctx);
        out.push_str("format!(\""); out.push_str(&fmt.template); out.push('"');
        for arg in &fmt.args { out.push_str(", "); out.push_str(arg); }
        out.push(')');
    } else {
        out.push('"');
        for p in parts { if let StringPart::Text(t) = p { out.push_str(t); } }
        out.push('"');
    }
}

fn gen_print(args: &[Expr], out: &mut String, ctx: &Ctx) {
    if args.len() == 1 {
        if let Expr::StringLiteral { parts, .. } = &args[0] {
            let fmt = build_format(parts, ctx);
            out.push_str("println!(\""); out.push_str(&fmt.template); out.push('"');
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

    #[test]
    fn test_hello() {
        let code = generate(&parser::parse("name = \"Мир\"\nprint(\"Привет, $name!\")").unwrap()).unwrap();
        assert!(code.contains("println!(\"Привет, {}!\", name)"));
    }

    #[test]
    fn test_fn_result() {
        let code = generate(&parser::parse("fn f(): Db\n    x = Sqlite.open(\"a\")?\n    return x\nend").unwrap()).unwrap();
        assert!(code.contains("-> Result<Db, Box<dyn std::error::Error + Send + Sync>>"));
        assert!(code.contains("return Ok(x)"));
    }

    #[test]
    fn test_fn_ref_params() {
        let code = generate(&parser::parse("fn f(db: Db, t: String)\n    print(t)\nend\nf(db, title)").unwrap()).unwrap();
        assert!(code.contains("fn f(db: &Db, t: &str)"));
        assert!(code.contains("f(&db, &title)"));
    }

    #[test]
    fn test_exec1() {
        let code = generate(&parser::parse("x = db.exec(\"sql\", val)").unwrap()).unwrap();
        assert!(code.contains("db.exec1(\"sql\", &val)"));
    }

    #[test]
    fn test_match_string() {
        let code = generate(&parser::parse("match cmd\n    \"a\": print(\"ok\")\n    _: print(\"no\")\nend").unwrap()).unwrap();
        assert!(code.contains("match cmd.as_str()"));
        assert!(code.contains("\"a\" =>"));
        assert!(code.contains("_ =>"));
    }

    #[test]
    fn test_spawn_mut_param_rejected() {
        let result = generate(&parser::parse("fn writer(::data)\n    print(data)\nend\nspawn writer(x)").unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot mutate external data"));
    }
}
