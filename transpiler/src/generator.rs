use std::collections::{HashMap, HashSet};
use crate::ast::*;

struct Ctx {
    structs: HashSet<String>,
    variant_to_enum: HashMap<String, String>,
    fn_params: HashMap<String, Vec<FnParam>>,
}

impl Ctx {
    fn from_program(program: &Program) -> Self {
        let mut ctx = Ctx { structs: HashSet::new(), variant_to_enum: HashMap::new(), fn_params: HashMap::new() };
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
        ctx
    }
}

fn map_type(t: &str) -> &str {
    match t { "Int" => "i64", "Float" => "f64", "String" => "String", "Bool" => "bool", o => o }
}

fn map_param_type(t: &str) -> String {
    match t {
        "Int" => "i64".into(), "Float" => "f64".into(), "Bool" => "bool".into(),
        "String" => "&str".into(), "Db" => "&Db".into(),
        other => format!("&{}", other),
    }
}

fn is_ref_type(t: &str) -> bool {
    !matches!(t, "Int" | "Float" | "Bool")
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
        Stmt::ForLoop { body, .. } => has_return_value(body),
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

pub fn generate(program: &Program) -> String {
    let ctx = Ctx::from_program(program);
    let mut out = String::new();
    out.push_str("#![allow(unused_mut, unused_variables, dead_code, unused_imports)]\n");
    out.push_str("use u_runtime::*;\n\n");

    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. } => {
                gen_stmt(stmt, &mut out, 0, &ctx, false);
                out.push('\n');
            }
            _ => {}
        }
    }

    out.push_str("fn main() {\n");
    out.push_str("    if let Err(e) = _u_main() { eprintln!(\"Ошибка: {}\", e); std::process::exit(1); }\n");
    out.push_str("}\n\n");
    out.push_str("fn _u_main() -> Result<(), Box<dyn std::error::Error>> {\n");
    for stmt in &program.statements {
        if !matches!(stmt, Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. }) {
            gen_stmt(stmt, &mut out, 1, &ctx, true);
        }
    }
    out.push_str("    Ok(())\n}\n");
    out
}

fn gen_stmts(stmts: &[Stmt], out: &mut String, indent: usize, ctx: &Ctx, result_fn: bool) {
    for stmt in stmts { gen_stmt(stmt, out, indent, ctx, result_fn); }
}

fn gen_stmt(stmt: &Stmt, out: &mut String, indent: usize, ctx: &Ctx, result_fn: bool) {
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
            out.push_str("let mut "); out.push_str(name); out.push_str(" = ");
            gen_expr(value, out, ctx); out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            gen_expr(expr, out, ctx); out.push_str(";\n");
        }
        Stmt::FnDef { name, params, return_type, body, .. } => {
            let fn_uses_q = uses_qmark(body);
            let has_ret = has_return_value(body);
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
                out.push_str(" -> Result<"); out.push_str(ret); out.push_str(", Box<dyn std::error::Error>>");
            } else if has_ret {
                let ret = return_type.as_deref().map(map_type).unwrap_or("i64");
                out.push_str(" -> "); out.push_str(ret);
            }
            out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx, fn_uses_q);
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
            gen_stmts(body, out, indent + 1, ctx, result_fn);
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::If { condition, body, elifs, else_body, .. } => {
            out.push_str("if "); gen_expr(condition, out, ctx); out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx, result_fn);
            out.push_str(&pad); out.push('}');
            for (cond, block) in elifs {
                out.push_str(" else if "); gen_expr(cond, out, ctx); out.push_str(" {\n");
                gen_stmts(block, out, indent + 1, ctx, result_fn);
                out.push_str(&pad); out.push('}');
            }
            if let Some(eb) = else_body {
                out.push_str(" else {\n"); gen_stmts(eb, out, indent + 1, ctx, result_fn);
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
                gen_stmt(&arm.body, out, indent + 2, ctx, result_fn);
                out.push_str(&pad); out.push_str("    }\n");
            }
            out.push_str(&pad); out.push_str("}\n");
        }
        Stmt::Return { value, .. } => {
            if result_fn {
                if let Some(val) = value {
                    out.push_str("return Ok("); gen_expr(val, out, ctx); out.push_str(");\n");
                } else {
                    out.push_str("return Ok(());\n");
                }
            } else {
                out.push_str("return");
                if let Some(val) = value { out.push(' '); gen_expr(val, out, ctx); }
                out.push_str(";\n");
            }
        }
        Stmt::MutAssign { object, field, value, .. } => {
            gen_expr(object, out, ctx); out.push('.'); out.push_str(field); out.push_str(" = ");
            gen_expr(value, out, ctx); out.push_str(";\n");
        }
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
            } else {
                gen_args(args, out, ctx);
            }
            out.push(')');
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
        let code = generate(&parser::parse("name = \"Мир\"\nprint(\"Привет, $name!\")").unwrap());
        assert!(code.contains("println!(\"Привет, {}!\", name)"));
    }

    #[test]
    fn test_fn_result() {
        let code = generate(&parser::parse("fn f(): Db\n    x = Sqlite.open(\"a\")?\n    return x\nend").unwrap());
        assert!(code.contains("-> Result<Db, Box<dyn std::error::Error>>"));
        assert!(code.contains("return Ok(x)"));
    }

    #[test]
    fn test_fn_ref_params() {
        let code = generate(&parser::parse("fn f(db: Db, t: String)\n    print(t)\nend\nf(db, title)").unwrap());
        assert!(code.contains("fn f(db: &Db, t: &str)"));
        assert!(code.contains("f(&db, &title)"));
    }

    #[test]
    fn test_exec1() {
        let code = generate(&parser::parse("x = db.exec(\"sql\", val)").unwrap());
        assert!(code.contains("db.exec1(\"sql\", &val)"));
    }

    #[test]
    fn test_match_string() {
        let code = generate(&parser::parse("match cmd\n    \"a\": print(\"ok\")\n    _: print(\"no\")\nend").unwrap());
        assert!(code.contains("match cmd.as_str()"));
        assert!(code.contains("\"a\" =>"));
        assert!(code.contains("_ =>"));
    }
}
