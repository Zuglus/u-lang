use std::collections::{HashMap, HashSet};

use crate::ast::*;

struct Ctx {
    structs: HashSet<String>,
    variant_to_enum: HashMap<String, String>,
}

impl Ctx {
    fn from_program(program: &Program) -> Self {
        let mut structs = HashSet::new();
        let mut variant_to_enum = HashMap::new();
        for stmt in &program.statements {
            match stmt {
                Stmt::StructDef { name, .. } => { structs.insert(name.clone()); }
                Stmt::TypeDef { name, variants, .. } => {
                    for v in variants { variant_to_enum.insert(v.name.clone(), name.clone()); }
                }
                _ => {}
            }
        }
        Ctx { structs, variant_to_enum }
    }
}

fn map_type(t: &str) -> &str {
    match t { "Int" => "i64", "Float" => "f64", "String" => "String", "Bool" => "bool", o => o }
}

pub fn generate(program: &Program) -> String {
    let ctx = Ctx::from_program(program);
    let mut out = String::new();
    out.push_str("#![allow(unused_mut, unused_variables, dead_code)]\n\n");

    // struct/type/fn defs go outside main
    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. } => {
                gen_stmt(stmt, &mut out, 0, &ctx);
                out.push('\n');
            }
            _ => {}
        }
    }

    out.push_str("fn main() {\n");
    for stmt in &program.statements {
        if !matches!(stmt, Stmt::StructDef { .. } | Stmt::TypeDef { .. } | Stmt::FnDef { .. }) {
            gen_stmt(stmt, &mut out, 1, &ctx);
        }
    }
    out.push_str("}\n");
    out
}

fn gen_stmts(stmts: &[Stmt], out: &mut String, indent: usize, ctx: &Ctx) {
    for stmt in stmts { gen_stmt(stmt, out, indent, ctx); }
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
                        if let Some(en) = ctx.variant_to_enum.get(&arm.pattern.variant) {
                            return en.clone();
                        }
                    }
                }
            }
        }
    }
    "i64".to_string()
}

fn gen_stmt(stmt: &Stmt, out: &mut String, indent: usize, ctx: &Ctx) {
    let pad = "    ".repeat(indent);
    out.push_str(&pad);
    match stmt {
        Stmt::StructDef { name, fields, .. } => {
            out.push_str("#[derive(Debug, Clone)]\nstruct ");
            out.push_str(name);
            out.push_str(" {\n");
            for f in fields {
                out.push_str(&pad);
                out.push_str("    ");
                out.push_str(&f.name);
                out.push_str(": ");
                out.push_str(map_type(&f.type_name));
                out.push_str(",\n");
            }
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Stmt::TypeDef { name, variants, .. } => {
            out.push_str("#[derive(Debug, Clone)]\nenum ");
            out.push_str(name);
            out.push_str(" {\n");
            for v in variants {
                out.push_str(&pad);
                out.push_str("    ");
                out.push_str(&v.name);
                out.push('(');
                for (i, f) in v.fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(map_type(&f.type_name));
                }
                out.push_str("),\n");
            }
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Stmt::Assignment { name, value, .. } => {
            out.push_str("let mut ");
            out.push_str(name);
            out.push_str(" = ");
            gen_expr(value, out, ctx);
            out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            gen_expr(expr, out, ctx);
            out.push_str(";\n");
        }
        Stmt::FnDef { name, params, body, .. } => {
            out.push_str("fn ");
            out.push_str(name);
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                out.push_str(p);
                out.push_str(": ");
                out.push_str(&infer_param_type(p, body, ctx));
            }
            out.push(')');
            if has_return_value(body) { out.push_str(" -> i64"); }
            out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Stmt::ForLoop { pattern, iter, body, .. } => {
            out.push_str("for ");
            match pattern {
                ForPattern::Single(n) => out.push_str(n),
                ForPattern::Tuple(ns) => { out.push('('); out.push_str(&ns.join(", ")); out.push(')'); }
            }
            out.push_str(" in ");
            gen_expr(iter, out, ctx);
            out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Stmt::If { condition, body, elifs, else_body, .. } => {
            out.push_str("if ");
            gen_expr(condition, out, ctx);
            out.push_str(" {\n");
            gen_stmts(body, out, indent + 1, ctx);
            out.push_str(&pad);
            out.push('}');
            for (cond, block) in elifs {
                out.push_str(" else if ");
                gen_expr(cond, out, ctx);
                out.push_str(" {\n");
                gen_stmts(block, out, indent + 1, ctx);
                out.push_str(&pad);
                out.push('}');
            }
            if let Some(eb) = else_body {
                out.push_str(" else {\n");
                gen_stmts(eb, out, indent + 1, ctx);
                out.push_str(&pad);
                out.push('}');
            }
            out.push('\n');
        }
        Stmt::Match { expr, arms, .. } => {
            out.push_str("match ");
            gen_expr(expr, out, ctx);
            out.push_str(" {\n");
            for arm in arms {
                out.push_str(&pad);
                out.push_str("    ");
                // Qualify variant with enum name
                if let Some(en) = ctx.variant_to_enum.get(&arm.pattern.variant) {
                    out.push_str(en);
                    out.push_str("::");
                }
                out.push_str(&arm.pattern.variant);
                out.push('(');
                out.push_str(&arm.pattern.bindings.join(", "));
                out.push_str(") => {\n");
                gen_stmt(&arm.body, out, indent + 2, ctx);
                out.push_str(&pad);
                out.push_str("    }\n");
            }
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Stmt::Return { value, .. } => {
            out.push_str("return");
            if let Some(val) = value { out.push(' '); gen_expr(val, out, ctx); }
            out.push_str(";\n");
        }
        Stmt::MutAssign { object, field, value, .. } => {
            gen_expr(object, out, ctx);
            out.push('.');
            out.push_str(field);
            out.push_str(" = ");
            gen_expr(value, out, ctx);
            out.push_str(";\n");
        }
    }
}

fn gen_expr(expr: &Expr, out: &mut String, ctx: &Ctx) {
    match expr {
        Expr::StringLiteral { parts, .. } => gen_string(parts, out, ctx),
        Expr::IntLiteral { value, .. } => { out.push_str(&value.to_string()); out.push_str("_i64"); }
        Expr::FloatLiteral { value, .. } => {
            let s = value.to_string();
            out.push_str(&s);
            if !s.contains('.') { out.push_str(".0"); }
            out.push_str("_f64");
        }
        Expr::BoolLiteral { value, .. } => out.push_str(if *value { "true" } else { "false" }),
        Expr::Identifier { name, .. } => out.push_str(name),
        Expr::FunctionCall { name, args, .. } => {
            if name == "print" { gen_print(args, out, ctx); }
            else { out.push_str(name); out.push('('); gen_args(args, out, ctx); out.push(')'); }
        }
        Expr::Lambda { params, body, .. } => {
            out.push('|'); out.push_str(&params.join(", ")); out.push_str("| ");
            gen_expr(body, out, ctx);
        }
        Expr::MethodCall { object, method, args, .. } => {
            gen_expr(object, out, ctx); out.push('.'); out.push_str(method);
            out.push('('); gen_args(args, out, ctx); out.push(')');
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
                // Rust struct init with named fields
                out.push_str(name);
                out.push_str(" { ");
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(fname);
                    out.push_str(": ");
                    gen_expr(fval, out, ctx);
                }
                out.push_str(" }");
            } else if ctx.variant_to_enum.contains_key(name) {
                // Enum variant — positional construction
                let en = &ctx.variant_to_enum[name];
                out.push_str(en);
                out.push_str("::");
                out.push_str(name);
                out.push('(');
                for (i, (_, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    gen_expr(fval, out, ctx);
                }
                out.push(')');
            } else {
                // Unknown — treat as function call with positional args
                out.push_str(name);
                out.push('(');
                for (i, (_, fval)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    gen_expr(fval, out, ctx);
                }
                out.push(')');
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
    } else if args.is_empty() {
        out.push_str("println!()");
    } else {
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
                let mut arg = String::new();
                gen_expr(expr, &mut arg, ctx);
                args.push(arg);
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
        let ast = parser::parse("name = \"Мир\"\nprint(\"Привет, $name!\")").unwrap();
        let code = generate(&ast);
        assert!(code.contains("let mut name = \"Мир\""));
        assert!(code.contains("println!(\"Привет, {}!\", name)"));
    }

    #[test]
    fn test_struct_gen() {
        let ast = parser::parse("struct Point\n    x: Int\n    y: Int\nend").unwrap();
        let code = generate(&ast);
        assert!(code.contains("struct Point {"));
        assert!(code.contains("x: i64,"));
    }

    #[test]
    fn test_enum_gen() {
        let ast = parser::parse("type Shape\n    Circle(r: Int)\nend").unwrap();
        let code = generate(&ast);
        assert!(code.contains("enum Shape {"));
        assert!(code.contains("Circle(i64)"));
    }

    #[test]
    fn test_struct_init_gen() {
        let ast = parser::parse("struct P\n    x: Int\nend\np = P(x: 1)").unwrap();
        let code = generate(&ast);
        assert!(code.contains("P { x: 1_i64 }"));
    }

    #[test]
    fn test_variant_init_gen() {
        let ast = parser::parse("type S\n    C(r: Int)\nend\nx = C(r: 5)").unwrap();
        let code = generate(&ast);
        assert!(code.contains("S::C(5_i64)"));
    }

    #[test]
    fn test_match_gen() {
        let src = "type S\n    A(x: Int)\nend\nmatch s\n    A(v): return v\nend";
        let ast = parser::parse(src).unwrap();
        let code = generate(&ast);
        assert!(code.contains("S::A(v) =>"));
    }

    #[test]
    fn test_mutation_gen() {
        let src = "struct P\n    x: Int\nend\np = P(x: 1)\np::x = 99";
        let ast = parser::parse(src).unwrap();
        let code = generate(&ast);
        assert!(code.contains("p.x = 99_i64"));
    }

    #[test]
    fn test_fn_with_match_infers_type() {
        let src = "type S\n    A(x: Int)\nend\nfn f(s)\n    match s\n        A(v): return v\n    end\nend";
        let ast = parser::parse(src).unwrap();
        let code = generate(&ast);
        assert!(code.contains("fn f(s: S) -> i64"));
    }

    #[test]
    fn test_list_gen() {
        let ast = parser::parse("x = [1, 2, 3]").unwrap();
        let code = generate(&ast);
        assert!(code.contains("vec![1_i64, 2_i64, 3_i64]"));
    }

    #[test]
    fn test_for_gen() {
        let ast = parser::parse("for x in items\n    print(x)\nend").unwrap();
        let code = generate(&ast);
        assert!(code.contains("for x in items {"));
    }

    #[test]
    fn test_return_gen() {
        let ast = parser::parse("fn id(x)\n    return x\nend").unwrap();
        let code = generate(&ast);
        assert!(code.contains("return x;"));
    }
}
