use pest::Parser;

use crate::ast::*;

#[derive(pest_derive::Parser)]
#[grammar = "src/u.pest"]
struct UParser;

pub fn parse(source: &str) -> anyhow::Result<Program> {
    let mut pairs = UParser::parse(Rule::program, source)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let program_pair = pairs.next().unwrap();
    build_program(program_pair)
}

fn span(s: pest::Span) -> Span {
    Span { start: s.start(), end: s.end() }
}

fn is_kw(rule: Rule) -> bool {
    matches!(rule,
        Rule::fn_kw | Rule::for_kw | Rule::in_kw | Rule::if_kw
        | Rule::elif_kw | Rule::else_kw | Rule::end_kw | Rule::return_kw
        | Rule::struct_kw | Rule::type_kw | Rule::match_kw
    )
}

fn meaningful(pairs: pest::iterators::Pairs<Rule>) -> impl Iterator<Item = pest::iterators::Pair<Rule>> + '_ {
    pairs.filter(|p| !is_kw(p.as_rule()))
}

fn reparse_expression(raw: &str) -> anyhow::Result<Expr> {
    let mut pairs = UParser::parse(Rule::expression_entry, raw)
        .map_err(|e| anyhow::anyhow!("interpolation parse error: {}", e))?;
    let entry = pairs.next().unwrap();
    let expr_pair = entry.into_inner().find(|p| p.as_rule() == Rule::expression).unwrap();
    build_expression(expr_pair)
}

fn build_program(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Program> {
    let mut statements = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::statement {
            statements.push(build_statement(inner)?);
        }
    }
    Ok(Program { statements })
}

fn build_block(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Vec<Stmt>> {
    let mut stmts = Vec::new();
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::statement {
            stmts.push(build_statement(inner)?);
        }
    }
    Ok(stmts)
}

fn build_statement(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Stmt> {
    let inner = pair.into_inner().next().unwrap();
    build_stmt_inner(inner)
}

fn build_stmt_inner(inner: pest::iterators::Pair<Rule>) -> anyhow::Result<Stmt> {
    match inner.as_rule() {
        Rule::assignment => {
            let s = span(inner.as_span());
            let mut parts = inner.into_inner();
            let name = parts.next().unwrap().as_str().to_string();
            let value = build_expression(parts.next().unwrap())?;
            Ok(Stmt::Assignment { name, value, span: s })
        }
        Rule::expr_stmt => {
            let s = span(inner.as_span());
            let expr = build_expression(inner.into_inner().next().unwrap())?;
            Ok(Stmt::ExprStmt { expr, span: s })
        }
        Rule::fn_def => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let name = parts.next().unwrap().as_str().to_string();
            let mut params = Vec::new();
            let next = parts.next().unwrap();
            let block = if next.as_rule() == Rule::param_list {
                for p in next.into_inner() { params.push(p.as_str().to_string()); }
                parts.next().unwrap()
            } else { next };
            let body = build_block(block)?;
            Ok(Stmt::FnDef { name, params, body, span: s })
        }
        Rule::for_loop => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let pattern = build_for_pattern(parts.next().unwrap())?;
            let iter = build_expression(parts.next().unwrap())?;
            let body = build_block(parts.next().unwrap())?;
            Ok(Stmt::ForLoop { pattern, iter, body, span: s })
        }
        Rule::if_stmt => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let condition = build_expression(parts.next().unwrap())?;
            let body = build_block(parts.next().unwrap())?;
            let mut elifs = Vec::new();
            let mut else_body = None;
            for part in parts {
                match part.as_rule() {
                    Rule::elif_clause => {
                        let mut ei = meaningful(part.into_inner());
                        let cond = build_expression(ei.next().unwrap())?;
                        let block = build_block(ei.next().unwrap())?;
                        elifs.push((cond, block));
                    }
                    Rule::else_clause => {
                        let block = meaningful(part.into_inner()).next().unwrap();
                        else_body = Some(build_block(block)?);
                    }
                    _ => {}
                }
            }
            Ok(Stmt::If { condition, body, elifs, else_body, span: s })
        }
        Rule::return_stmt => {
            let s = span(inner.as_span());
            let value = meaningful(inner.into_inner()).next()
                .map(|e| build_expression(e)).transpose()?;
            Ok(Stmt::Return { value, span: s })
        }
        Rule::struct_def => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let name = parts.next().unwrap().as_str().to_string();
            let mut fields = Vec::new();
            for p in parts {
                if p.as_rule() == Rule::typed_field {
                    let mut fi = p.into_inner();
                    let fname = fi.next().unwrap().as_str().to_string();
                    let tname = fi.next().unwrap().as_str().to_string();
                    fields.push(TypedField { name: fname, type_name: tname });
                }
            }
            Ok(Stmt::StructDef { name, fields, span: s })
        }
        Rule::type_def => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let name = parts.next().unwrap().as_str().to_string();
            let mut variants = Vec::new();
            for p in parts {
                if p.as_rule() == Rule::type_variant {
                    let mut vi = p.into_inner();
                    let vname = vi.next().unwrap().as_str().to_string();
                    let mut fields = Vec::new();
                    for f in vi {
                        if f.as_rule() == Rule::typed_field {
                            let mut fi = f.into_inner();
                            let fname = fi.next().unwrap().as_str().to_string();
                            let tname = fi.next().unwrap().as_str().to_string();
                            fields.push(TypedField { name: fname, type_name: tname });
                        }
                    }
                    variants.push(Variant { name: vname, fields });
                }
            }
            Ok(Stmt::TypeDef { name, variants, span: s })
        }
        Rule::match_stmt => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let expr = build_expression(parts.next().unwrap())?;
            let mut arms = Vec::new();
            for p in parts {
                if p.as_rule() == Rule::match_arm {
                    arms.push(build_match_arm(p)?);
                }
            }
            Ok(Stmt::Match { expr, arms, span: s })
        }
        Rule::mutation_stmt => {
            let s = span(inner.as_span());
            let mut parts = inner.into_inner();
            let object = build_expression(parts.next().unwrap())?;
            let field = parts.next().unwrap().as_str().to_string();
            let value = build_expression(parts.next().unwrap())?;
            Ok(Stmt::MutAssign { object, field, value, span: s })
        }
        _ => unreachable!("unexpected rule in statement: {:?}", inner.as_rule()),
    }
}

fn build_match_arm(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<MatchArm> {
    let mut inner = pair.into_inner();
    let pattern_pair = inner.next().unwrap();
    let mut pi = pattern_pair.into_inner();
    let variant = pi.next().unwrap().as_str().to_string();
    let bindings = pi.next()
        .map(|b| b.into_inner().map(|i| i.as_str().to_string()).collect())
        .unwrap_or_default();
    let pattern = MatchPattern { variant, bindings };

    let body_pair = inner.next().unwrap();
    let body = build_stmt_inner(body_pair)?;
    Ok(MatchArm { pattern, body })
}

fn build_for_pattern(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<ForPattern> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::identifier => Ok(ForPattern::Single(inner.as_str().to_string())),
        Rule::tuple_pattern => {
            let names = inner.into_inner().map(|p| p.as_str().to_string()).collect();
            Ok(ForPattern::Tuple(names))
        }
        _ => unreachable!(),
    }
}

fn build_expression(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Expr> {
    match pair.as_rule() {
        Rule::expression => build_expression(pair.into_inner().next().unwrap()),
        Rule::comparison => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let left = build_expression(inner.next().unwrap())?;
            if let Some(op_pair) = inner.next() {
                let right = build_expression(inner.next().unwrap())?;
                Ok(Expr::BinaryOp { left: Box::new(left), op: op_pair.as_str().to_string(), right: Box::new(right), span: s })
            } else { Ok(left) }
        }
        Rule::addition | Rule::multiplication => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let mut expr = build_expression(inner.next().unwrap())?;
            while let Some(op_pair) = inner.next() {
                let right = build_expression(inner.next().unwrap())?;
                expr = Expr::BinaryOp { left: Box::new(expr), op: op_pair.as_str().to_string(), right: Box::new(right), span: s.clone() };
            }
            Ok(expr)
        }
        Rule::unary => {
            let mut parts: Vec<_> = pair.into_inner().collect();
            let operand_pair = parts.pop().unwrap();
            let mut expr = build_expression(operand_pair)?;
            for prefix in parts.into_iter().rev() {
                let s = span(prefix.as_span());
                expr = Expr::UnaryOp { op: prefix.as_str().to_string(), expr: Box::new(expr), span: s };
            }
            Ok(expr)
        }
        Rule::operand => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let mut expr = build_expression(inner.next().unwrap())?;
            for part in inner {
                match part.as_rule() {
                    Rule::method_call => {
                        let mc_span = span(part.as_span());
                        let mut mc = part.into_inner();
                        let method = mc.next().unwrap().as_str().to_string();
                        let mut args = Vec::new();
                        if let Some(al) = mc.next() { for a in al.into_inner() { args.push(build_expression(a)?); } }
                        expr = Expr::MethodCall { object: Box::new(expr), method, args, span: mc_span };
                    }
                    Rule::field_access => {
                        let fa_span = span(part.as_span());
                        let field = part.into_inner().next().unwrap().as_str().to_string();
                        expr = Expr::FieldAccess { object: Box::new(expr), field, span: fa_span };
                    }
                    Rule::postfix_op => {
                        expr = Expr::PostfixOp { span: Span { start: s.start, end: s.end }, expr: Box::new(expr), op: part.as_str().to_string() };
                    }
                    _ => {}
                }
            }
            Ok(expr)
        }
        Rule::primary => build_expression(pair.into_inner().next().unwrap()),
        Rule::lambda => {
            let s = span(pair.as_span());
            let mut inner = meaningful(pair.into_inner());
            let mut params = Vec::new();
            let first = inner.next().unwrap();
            let body_pair = if first.as_rule() == Rule::param_list {
                for p in first.into_inner() { params.push(p.as_str().to_string()); }
                inner.next().unwrap()
            } else { first };
            Ok(Expr::Lambda { params, body: Box::new(build_expression(body_pair)?), span: s })
        }
        Rule::struct_init => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let nargs = inner.next().unwrap(); // named_arg_list
            let mut fields = Vec::new();
            for na in nargs.into_inner() {
                let mut nai = na.into_inner();
                let fname = nai.next().unwrap().as_str().to_string();
                let fval = build_expression(nai.next().unwrap())?;
                fields.push((fname, fval));
            }
            Ok(Expr::StructInit { name, fields, span: s })
        }
        Rule::function_call => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let mut args = Vec::new();
            if let Some(al) = inner.next() { for a in al.into_inner() { args.push(build_expression(a)?); } }
            Ok(Expr::FunctionCall { name, args, span: s })
        }
        Rule::paren_expr => build_expression(pair.into_inner().next().unwrap()),
        Rule::list_literal => {
            let s = span(pair.as_span());
            let elements = pair.into_inner().map(|e| build_expression(e)).collect::<Result<_, _>>()?;
            Ok(Expr::List { elements, span: s })
        }
        Rule::string_literal => {
            let s = span(pair.as_span());
            let mut parts = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::string_part {
                    let part = inner.into_inner().next().unwrap();
                    match part.as_rule() {
                        Rule::string_text => parts.push(StringPart::Text(part.as_str().to_string())),
                        Rule::interp_var => {
                            let ident = part.into_inner().next().unwrap();
                            parts.push(StringPart::Interpolation(Expr::Identifier {
                                name: ident.as_str().to_string(), span: span(ident.as_span()),
                            }));
                        }
                        Rule::interp_expr => {
                            let raw = part.into_inner().next().unwrap();
                            parts.push(StringPart::Interpolation(reparse_expression(raw.as_str())?));
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Ok(Expr::StringLiteral { parts, span: s })
        }
        Rule::number => {
            let s = span(pair.as_span());
            let text = pair.as_str();
            if text.contains('.') {
                Ok(Expr::FloatLiteral { value: text.parse()?, span: s })
            } else {
                Ok(Expr::IntLiteral { value: text.parse()?, span: s })
            }
        }
        Rule::bool_literal => Ok(Expr::BoolLiteral { value: pair.as_str() == "true", span: span(pair.as_span()) }),
        Rule::identifier => Ok(Expr::Identifier { name: pair.as_str().to_string(), span: span(pair.as_span()) }),
        _ => unreachable!("unexpected rule: {:?}", pair.as_rule()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hello() {
        let p = parse("name = \"Мир\"\nprint(\"Привет, $name!\")").unwrap();
        assert_eq!(p.statements.len(), 2);
    }

    #[test]
    fn test_struct_def() {
        let p = parse("struct Point\n    x: Int\n    y: Int\nend").unwrap();
        match &p.statements[0] {
            Stmt::StructDef { name, fields, .. } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "x");
                assert_eq!(fields[0].type_name, "Int");
            }
            _ => panic!("expected struct def"),
        }
    }

    #[test]
    fn test_type_def() {
        let p = parse("type Shape\n    Circle(radius: Int)\n    Rect(width: Int, height: Int)\nend").unwrap();
        match &p.statements[0] {
            Stmt::TypeDef { name, variants, .. } => {
                assert_eq!(name, "Shape");
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Circle");
                assert_eq!(variants[1].fields.len(), 2);
            }
            _ => panic!("expected type def"),
        }
    }

    #[test]
    fn test_struct_init() {
        let p = parse("p = Point(x: 10, y: 20)").unwrap();
        match &p.statements[0] {
            Stmt::Assignment { value, .. } => match value {
                Expr::StructInit { name, fields, .. } => {
                    assert_eq!(name, "Point");
                    assert_eq!(fields.len(), 2);
                    assert_eq!(fields[0].0, "x");
                }
                _ => panic!("expected struct init"),
            },
            _ => panic!("expected assignment"),
        }
    }

    #[test]
    fn test_match_stmt() {
        let src = "match shape\n    Circle(r): return r\n    Rect(w, h): return w\nend";
        let p = parse(src).unwrap();
        match &p.statements[0] {
            Stmt::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                assert_eq!(arms[0].pattern.variant, "Circle");
                assert_eq!(arms[0].pattern.bindings, vec!["r"]);
                assert_eq!(arms[1].pattern.bindings, vec!["w", "h"]);
            }
            _ => panic!("expected match"),
        }
    }

    #[test]
    fn test_mutation() {
        let p = parse("p::x = 100").unwrap();
        match &p.statements[0] {
            Stmt::MutAssign { field, .. } => assert_eq!(field, "x"),
            _ => panic!("expected mutation"),
        }
    }

    #[test]
    fn test_shapes_example() {
        let src = r#"struct Point
    x: Int
    y: Int
end

type Shape
    Circle(radius: Int)
    Rect(width: Int, height: Int)
end

fn area(shape)
    match shape
        Circle(r): return r * r * 3
        Rect(w, h): return w * h
    end
end

p = Point(x: 10, y: 20)
print("Point: $(p.x), $(p.y)")
p::x = 100
print("After: $(p.x)")
print("Circle: $(area(Circle(radius: 5)))")
print("Rect: $(area(Rect(width: 3, height: 4)))")"#;
        let p = parse(src).unwrap();
        assert_eq!(p.statements.len(), 9);
    }

    #[test]
    fn test_binary_ops() {
        let p = parse("x = 1 + 2 * 3").unwrap();
        match &p.statements[0] {
            Stmt::Assignment { value, .. } => {
                assert!(matches!(value, Expr::BinaryOp { op, .. } if op == "+"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_report_parses() {
        let src = r#"data = parse_csv(read_file("workers.csv")?)

by_dept = data
    .group_by(fn(row) row.string("department"))
    .map_values(fn(rows) rows.avg(fn(r) r.float("salary")))

for (dept, avg) in by_dept
    print("$dept: средняя зарплата $(avg.round(2))")
end"#;
        let p = parse(src).unwrap();
        assert_eq!(p.statements.len(), 3);
    }

    #[test]
    fn test_calc_parses() {
        let src = r#"fn abs(x)
    if x < 0
        return 0 - x
    end
    return x
end

print("abs(-42) = $(abs(-42))")

for n in [1, 2, 3, 4, 5]
    print("$(n) * $(n) = $(n * n)")
end"#;
        let p = parse(src).unwrap();
        assert_eq!(p.statements.len(), 3);
    }
}
