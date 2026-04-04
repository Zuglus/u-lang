use pest::Parser;
use crate::ast::*;

#[derive(pest_derive::Parser)]
#[grammar = "src/u.pest"]
struct UParser;

pub fn parse(source: &str) -> anyhow::Result<Program> {
    let mut pairs = UParser::parse(Rule::program, source)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    build_program(pairs.next().unwrap())
}

fn span(s: pest::Span) -> Span { Span { start: s.start(), end: s.end() } }

fn is_kw(rule: Rule) -> bool {
    matches!(rule, Rule::fn_kw | Rule::for_kw | Rule::in_kw | Rule::if_kw
        | Rule::elif_kw | Rule::else_kw | Rule::end_kw | Rule::return_kw
        | Rule::struct_kw | Rule::enum_kw | Rule::match_kw
        | Rule::spawn_kw | Rule::loop_kw | Rule::memory_kw | Rule::use_kw
        | Rule::trait_kw | Rule::impl_kw | Rule::test_kw | Rule::mut_kw
        | Rule::break_kw | Rule::continue_kw | Rule::while_kw | Rule::pub_kw
        | Rule::and_kw | Rule::or_kw | Rule::not_kw)
}

fn meaningful(pairs: pest::iterators::Pairs<'_, Rule>) -> impl Iterator<Item = pest::iterators::Pair<'_, Rule>> + '_ {
    pairs.filter(|p| !is_kw(p.as_rule()))
}

fn reparse_expression(raw: &str) -> anyhow::Result<Expr> {
    let mut pairs = UParser::parse(Rule::expression_entry, raw)
        .map_err(|e| anyhow::anyhow!("interpolation: {}", e))?;
    let entry = pairs.next().unwrap();
    build_expression(entry.into_inner().find(|p| p.as_rule() == Rule::expression).unwrap())
}

fn build_program(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Program> {
    let mut statements = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::statement { statements.push(build_statement(p)?); }
    }
    Ok(Program { statements })
}

fn build_block(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Vec<Stmt>> {
    let mut stmts = Vec::new();
    for p in pair.into_inner() {
        if p.as_rule() == Rule::statement { stmts.push(build_statement(p)?); }
    }
    Ok(stmts)
}

fn build_statement(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Stmt> {
    build_stmt_inner(pair.into_inner().next().unwrap())
}

fn build_stmt_inner(inner: pest::iterators::Pair<Rule>) -> anyhow::Result<Stmt> {
    match inner.as_rule() {
        Rule::assignment => {
            let s = span(inner.as_span());
            let mut p = inner.into_inner();
            let name = p.next().unwrap().as_str().to_string();
            Ok(Stmt::Assignment { name, value: build_expression(p.next().unwrap())?, span: s })
        }
        Rule::expr_stmt => {
            let s = span(inner.as_span());
            Ok(Stmt::ExprStmt { expr: build_expression(inner.into_inner().next().unwrap())?, span: s })
        }
        Rule::fn_def => {
            let s = span(inner.as_span());
            let mut parts = meaningful(inner.into_inner());
            let name = parts.next().unwrap().as_str().to_string();
            let mut params = Vec::new();
            let mut return_type = None;
            let mut block_pair = None;
            for p in parts {
                match p.as_rule() {
                    Rule::fn_params => {
                        for fp in p.into_inner() {
                            let mut fi = fp.into_inner();
                            let mut is_mut = false;
                            let first = fi.next().unwrap();
                            let pname;
                            if first.as_rule() == Rule::mut_marker {
                                is_mut = true;
                                pname = fi.next().unwrap().as_str().to_string();
                            } else {
                                pname = first.as_str().to_string();
                            }
                            let ptype = fi.next().map(|t| t.as_str().to_string());
                            params.push(FnParam { name: pname, type_ann: ptype, is_mut });
                        }
                    }
                    Rule::fn_ret => {
                        return_type = Some(p.into_inner().next().unwrap().as_str().to_string());
                    }
                    Rule::block => { block_pair = Some(p); }
                    _ => {}
                }
            }
            let body = build_block(block_pair.unwrap())?;
            Ok(Stmt::FnDef { name, params, return_type, body, is_test: false, is_pub: false, span: s })
        }
        Rule::test_fn_def => {
            let s = span(inner.as_span());
            let fn_pair = inner.into_inner().find(|p| p.as_rule() == Rule::fn_def).unwrap();
            match build_stmt_inner(fn_pair)? {
                Stmt::FnDef { name, params, return_type, body, .. } =>
                    Ok(Stmt::FnDef { name, params, return_type, body, is_test: true, is_pub: false, span: s }),
                _ => unreachable!(),
            }
        }
        Rule::pub_fn_def => {
            let s = span(inner.as_span());
            let fn_pair = inner.into_inner().find(|p| p.as_rule() == Rule::fn_def).unwrap();
            match build_stmt_inner(fn_pair)? {
                Stmt::FnDef { name, params, return_type, body, .. } =>
                    Ok(Stmt::FnDef { name, params, return_type, body, is_test: false, is_pub: true, span: s }),
                _ => unreachable!(),
            }
        }
        Rule::for_loop => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let pattern = build_for_pattern(p.next().unwrap())?;
            let iter = build_expression(p.next().unwrap())?;
            let body = build_block(p.next().unwrap())?;
            Ok(Stmt::ForLoop { pattern, iter, body, span: s })
        }
        Rule::if_stmt => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let condition = build_expression(p.next().unwrap())?;
            let body = build_block(p.next().unwrap())?;
            let mut elifs = Vec::new();
            let mut else_body = None;
            for part in p {
                match part.as_rule() {
                    Rule::elif_clause => {
                        let mut ei = meaningful(part.into_inner());
                        elifs.push((build_expression(ei.next().unwrap())?, build_block(ei.next().unwrap())?));
                    }
                    Rule::else_clause => {
                        else_body = Some(build_block(meaningful(part.into_inner()).next().unwrap())?);
                    }
                    _ => {}
                }
            }
            Ok(Stmt::If { condition, body, elifs, else_body, span: s })
        }
        Rule::return_stmt => {
            let s = span(inner.as_span());
            let value = meaningful(inner.into_inner()).next().map(|e| build_expression(e)).transpose()?;
            Ok(Stmt::Return { value, span: s })
        }
        Rule::struct_def => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner()).peekable();
            let name = p.next().unwrap().as_str().to_string();
            let type_params = if let Some(tp) = p.peek() {
                if tp.as_rule() == Rule::type_params {
                    p.next().unwrap().into_inner().filter(|x| x.as_rule() == Rule::identifier).map(|id| id.as_str().to_string()).collect()
                } else { Vec::new() }
            } else { Vec::new() };
            let fields = p.filter(|x| x.as_rule() == Rule::typed_field).map(|tf| {
                let mut fi = tf.into_inner();
                TypedField { name: fi.next().unwrap().as_str().to_string(), type_name: fi.next().unwrap().as_str().to_string() }
            }).collect();
            Ok(Stmt::StructDef { name, type_params, fields, is_pub: false, span: s })
        }
        Rule::pub_struct_def => {
            let s = span(inner.as_span());
            let struct_pair = inner.into_inner().find(|p| p.as_rule() == Rule::struct_def).unwrap();
            match build_stmt_inner(struct_pair)? {
                Stmt::StructDef { name, type_params, fields, .. } =>
                    Ok(Stmt::StructDef { name, type_params, fields, is_pub: true, span: s }),
                _ => unreachable!(),
            }
        }
        Rule::type_def => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner()).peekable();
            let name = p.next().unwrap().as_str().to_string();
            let type_params = if let Some(tp) = p.peek() {
                if tp.as_rule() == Rule::type_params {
                    p.next().unwrap().into_inner().filter(|x| x.as_rule() == Rule::identifier).map(|id| id.as_str().to_string()).collect()
                } else { Vec::new() }
            } else { Vec::new() };
            let variants = p.filter(|x| x.as_rule() == Rule::type_variant).map(|vp| {
                let mut vi = vp.into_inner();
                let vname = vi.next().unwrap().as_str().to_string();
                let fields = vi.filter(|f| f.as_rule() == Rule::typed_field).map(|tf| {
                    let mut fi = tf.into_inner();
                    TypedField { name: fi.next().unwrap().as_str().to_string(), type_name: fi.next().unwrap().as_str().to_string() }
                }).collect();
                Variant { name: vname, fields }
            }).collect();
            Ok(Stmt::TypeDef { name, type_params, variants, is_pub: false, span: s })
        }
        Rule::pub_type_def => {
            let s = span(inner.as_span());
            let type_pair = inner.into_inner().find(|p| p.as_rule() == Rule::type_def).unwrap();
            match build_stmt_inner(type_pair)? {
                Stmt::TypeDef { name, type_params, variants, .. } =>
                    Ok(Stmt::TypeDef { name, type_params, variants, is_pub: true, span: s }),
                _ => unreachable!(),
            }
        }
        Rule::match_stmt => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let expr = build_expression(p.next().unwrap())?;
            let mut arms = Vec::new();
            for ap in p {
                if ap.as_rule() == Rule::match_arm { arms.push(build_match_arm(ap)?); }
            }
            Ok(Stmt::Match { expr, arms, span: s })
        }
        Rule::mutation_stmt => {
            let s = span(inner.as_span());
            let mut p = inner.into_inner();
            let obj_name = p.next().unwrap().as_str().to_string();
            let field = p.next().unwrap().as_str().to_string();
            let value = build_expression(p.next().unwrap())?;
            let object = Expr::Identifier { name: obj_name, span: s.clone() };
            Ok(Stmt::MutAssign { object, field, value, span: s })
        }
        Rule::while_loop => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let condition = build_expression(p.next().unwrap())?;
            let body = build_block(p.next().unwrap())?;
            Ok(Stmt::WhileLoop { condition, body, span: s })
        }
        Rule::break_stmt => {
            Ok(Stmt::Break { span: span(inner.as_span()) })
        }
        Rule::continue_stmt => {
            Ok(Stmt::Continue { span: span(inner.as_span()) })
        }
        Rule::spawn_stmt => {
            let s = span(inner.as_span());
            let expr = build_expression(meaningful(inner.into_inner()).next().unwrap())?;
            Ok(Stmt::Spawn { expr, span: s })
        }
        Rule::loop_stmt => {
            let s = span(inner.as_span());
            let block = meaningful(inner.into_inner()).next().unwrap();
            let body = build_block(block)?;
            Ok(Stmt::Loop { body, span: s })
        }
        Rule::memory_stmt => {
            let s = span(inner.as_span());
            let mode = meaningful(inner.into_inner()).next().unwrap().as_str().to_string();
            Ok(Stmt::MemoryDecl { mode, span: s })
        }
        Rule::use_stmt => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let path = p.next().unwrap().as_str().to_string();
            let imports = p.next().unwrap().into_inner()
                .map(|i| i.as_str().to_string())
                .collect();
            Ok(Stmt::UseDecl { path, imports, span: s })
        }
        Rule::trait_def => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let name = p.next().unwrap().as_str().to_string();
            let methods = p.filter(|x| x.as_rule() == Rule::trait_method_sig).map(|sig| {
                let mut si = meaningful(sig.into_inner());
                let mname = si.next().unwrap().as_str().to_string();
                let mut params = Vec::new();
                let mut return_type = None;
                for part in si {
                    match part.as_rule() {
                        Rule::fn_params => {
                            for fp in part.into_inner() {
                                let mut fi = fp.into_inner();
                                let mut is_mut = false;
                                let first = fi.next().unwrap();
                                let pname;
                                if first.as_rule() == Rule::mut_marker {
                                    is_mut = true;
                                    pname = fi.next().unwrap().as_str().to_string();
                                } else {
                                    pname = first.as_str().to_string();
                                }
                                let ptype = fi.next().map(|t| t.as_str().to_string());
                                params.push(FnParam { name: pname, type_ann: ptype, is_mut });
                            }
                        }
                        Rule::fn_ret => {
                            return_type = Some(part.into_inner().next().unwrap().as_str().to_string());
                        }
                        _ => {}
                    }
                }
                TraitMethodSig { name: mname, params, return_type }
            }).collect();
            Ok(Stmt::TraitDef { name, methods, span: s })
        }
        Rule::impl_block => {
            let s = span(inner.as_span());
            let mut p = meaningful(inner.into_inner());
            let first_name = p.next().unwrap().as_str().to_string();
            let mut trait_name = None;
            let mut target = first_name.clone();
            let mut methods = Vec::new();
            for part in p {
                match part.as_rule() {
                    Rule::identifier => {
                        trait_name = Some(first_name.clone());
                        target = part.as_str().to_string();
                    }
                    Rule::fn_def => {
                        methods.push(build_stmt_inner(part)?);
                    }
                    _ => {}
                }
            }
            Ok(Stmt::ImplBlock { trait_name, target, methods, span: s })
        }
        _ => unreachable!("unexpected: {:?}", inner.as_rule()),
    }
}

fn build_match_arm(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<MatchArm> {
    let mut inner = pair.into_inner();
    let pat_pair = inner.next().unwrap();
    let pattern = build_match_pattern(pat_pair)?;
    let body = build_stmt_inner(inner.next().unwrap())?;
    Ok(MatchArm { pattern, body })
}

fn build_match_pattern(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<MatchPattern> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::variant_pattern => {
            let mut vi = inner.into_inner();
            let first = vi.next().unwrap();
            match first.as_rule() {
                Rule::none_literal => {
                    Ok(MatchPattern::Variant { name: "None".to_string(), bindings: vec![] })
                }
                Rule::none_cap_pattern => {
                    Ok(MatchPattern::Variant { name: "None".to_string(), bindings: vec![] })
                }
                _ => {
                    let name = first.as_str().to_string();
                    let bindings = vi.next()
                        .map(|b| b.into_inner().map(|i| i.as_str().to_string()).collect())
                        .unwrap_or_default();
                    Ok(MatchPattern::Variant { name, bindings })
                }
            }
        }
        Rule::string_literal => {
            // Extract plain text from string literal parts
            let text: String = inner.into_inner()
                .filter(|p| p.as_rule() == Rule::string_part)
                .filter_map(|p| p.into_inner().next())
                .filter(|p| p.as_rule() == Rule::string_text)
                .map(|p| p.as_str().to_string())
                .collect();
            Ok(MatchPattern::StringLit(text))
        }
        Rule::match_wildcard => Ok(MatchPattern::Wildcard),
        Rule::list_pattern => {
            let inner_list = inner.into_inner().next().unwrap();
            match inner_list.as_rule() {
                Rule::list_empty => Ok(MatchPattern::List(ListPattern::Empty)),
                Rule::list_single => {
                    let binding = inner_list.into_inner().next().unwrap().as_str().to_string();
                    Ok(MatchPattern::List(ListPattern::Single(binding)))
                }
                Rule::list_cons => {
                    let mut parts = inner_list.into_inner();
                    let head = parts.next().unwrap().as_str().to_string();
                    let tail = parts.next().unwrap().as_str().to_string();
                    Ok(MatchPattern::List(ListPattern::Cons(head, tail)))
                }
                _ => unreachable!("unexpected list pattern: {:?}", inner_list.as_rule()),
            }
        }
        _ => unreachable!("unexpected match pattern: {:?}", inner.as_rule()),
    }
}

fn build_for_pattern(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<ForPattern> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::identifier => Ok(ForPattern::Single(inner.as_str().to_string())),
        Rule::tuple_pattern => Ok(ForPattern::Tuple(inner.into_inner().map(|p| p.as_str().to_string()).collect())),
        _ => unreachable!(),
    }
}

fn build_expression(pair: pest::iterators::Pair<Rule>) -> anyhow::Result<Expr> {
    match pair.as_rule() {
        Rule::expression => build_expression(pair.into_inner().next().unwrap()),
        Rule::logical => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let left = build_expression(inner.next().unwrap())?;
            match inner.next() {
                Some(op) => Ok(Expr::BinaryOp { left: Box::new(left), op: op.as_str().into(), right: Box::new(build_expression(inner.next().unwrap())?), span: s }),
                None => Ok(left),
            }
        }
        Rule::comparison => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let left = build_expression(inner.next().unwrap())?;
            match inner.next() {
                Some(op) => Ok(Expr::BinaryOp { left: Box::new(left), op: op.as_str().into(), right: Box::new(build_expression(inner.next().unwrap())?), span: s }),
                None => Ok(left),
            }
        }
        Rule::addition | Rule::multiplication => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let mut expr = build_expression(inner.next().unwrap())?;
            while let Some(op) = inner.next() {
                let right = build_expression(inner.next().unwrap())?;
                expr = Expr::BinaryOp { left: Box::new(expr), op: op.as_str().into(), right: Box::new(right), span: s.clone() };
            }
            Ok(expr)
        }
        Rule::unary => {
            let mut parts: Vec<_> = pair.into_inner().collect();
            let operand = parts.pop().unwrap();
            let mut expr = build_expression(operand)?;
            for prefix in parts.into_iter().rev() {
                expr = Expr::UnaryOp { op: prefix.as_str().into(), expr: Box::new(expr), span: span(prefix.as_span()) };
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
                        let is_mut = false;
                        let ms = span(part.as_span());
                        let mut mc = part.into_inner();
                        let method = mc.next().unwrap().as_str().to_string();
                        let args = mc.next().map(|al| al.into_inner().map(|a| build_expression(a)).collect::<Result<Vec<_>, _>>()).transpose()?.unwrap_or_default();
                        expr = Expr::MethodCall { object: Box::new(expr), method, args, is_mut, span: ms };
                    }
                    Rule::field_access => {
                        let fs = span(part.as_span());
                        expr = Expr::FieldAccess { object: Box::new(expr), field: part.into_inner().next().unwrap().as_str().into(), span: fs };
                    }
                    Rule::index_access => {
                        let is = span(part.as_span());
                        let index = build_expression(part.into_inner().next().unwrap())?;
                        expr = Expr::Index { object: Box::new(expr), index: Box::new(index), span: is };
                    }
                    Rule::postfix_op => {
                        expr = Expr::PostfixOp { span: Span { start: s.start, end: s.end }, expr: Box::new(expr), op: part.as_str().into() };
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
            let first = inner.next().unwrap();
            let (params, body_pair) = if first.as_rule() == Rule::param_list {
                (first.into_inner().map(|p| p.as_str().to_string()).collect(), inner.next().unwrap())
            } else { (vec![], first) };
            Ok(Expr::Lambda { params, body: Box::new(build_expression(body_pair)?), span: s })
        }
        Rule::struct_init => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let fields = inner.next().unwrap().into_inner().map(|na| {
                let mut nai = na.into_inner();
                let fname = nai.next().unwrap().as_str().to_string();
                let fval = build_expression(nai.next().unwrap()).unwrap();
                (fname, fval)
            }).collect();
            Ok(Expr::StructInit { name, fields, span: s })
        }
        Rule::function_call => {
            let s = span(pair.as_span());
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let args = inner.next().map(|al| al.into_inner().map(|a| build_expression(a)).collect::<Result<Vec<_>, _>>()).transpose()?.unwrap_or_default();
            Ok(Expr::FunctionCall { name, args, span: s })
        }
        Rule::paren_expr => build_expression(pair.into_inner().next().unwrap()),
        Rule::list_literal => {
            let s = span(pair.as_span());
            Ok(Expr::List { elements: pair.into_inner().map(|e| build_expression(e)).collect::<Result<_, _>>()?, span: s })
        }
        Rule::string_literal => {
            let s = span(pair.as_span());
            let mut parts = Vec::new();
            for p in pair.into_inner().filter(|p| p.as_rule() == Rule::string_part) {
                let inner = p.into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::string_text | Rule::dollar_lit => parts.push(StringPart::Text(inner.as_str().into())),
                    Rule::interp_var => {
                        let id = inner.into_inner().next().unwrap();
                        parts.push(StringPart::Interpolation(Expr::Identifier { name: id.as_str().into(), span: span(id.as_span()) }));
                    }
                    Rule::interp_expr => {
                        parts.push(StringPart::Interpolation(reparse_expression(inner.into_inner().next().unwrap().as_str())?));
                    }
                    _ => unreachable!(),
                }
            }
            Ok(Expr::StringLiteral { parts, span: s })
        }
        Rule::raw_string_literal => {
            let s = span(pair.as_span());
            let inner = pair.into_inner().next().unwrap();
            let part_rule = match inner.as_rule() {
                Rule::raw_string_1 => Rule::raw_1_part,
                Rule::raw_string_2 => Rule::raw_2_part,
                _ => unreachable!(),
            };
            let mut parts = Vec::new();
            for p in inner.into_inner().filter(|p| p.as_rule() == part_rule) {
                let pi = p.into_inner().next().unwrap();
                match pi.as_rule() {
                    Rule::raw_1_text | Rule::raw_2_text | Rule::dollar_lit =>
                        parts.push(StringPart::Text(pi.as_str().into())),
                    Rule::interp_var => {
                        let id = pi.into_inner().next().unwrap();
                        parts.push(StringPart::Interpolation(Expr::Identifier { name: id.as_str().into(), span: span(id.as_span()) }));
                    }
                    Rule::interp_expr => {
                        parts.push(StringPart::Interpolation(reparse_expression(pi.into_inner().next().unwrap().as_str())?));
                    }
                    _ => unreachable!("unexpected raw string part: {:?}", pi.as_rule()),
                }
            }
            Ok(Expr::StringLiteral { parts, span: s })
        }
        Rule::number => {
            let s = span(pair.as_span());
            let t = pair.as_str();
            if t.contains('.') { Ok(Expr::FloatLiteral { value: t.parse()?, span: s }) }
            else { Ok(Expr::IntLiteral { value: t.parse()?, span: s }) }
        }
        Rule::bool_literal => Ok(Expr::BoolLiteral { value: pair.as_str() == "true", span: span(pair.as_span()) }),
        Rule::none_literal => Ok(Expr::NoneLiteral { span: span(pair.as_span()) }),
        Rule::identifier => Ok(Expr::Identifier { name: pair.as_str().into(), span: span(pair.as_span()) }),
        _ => unreachable!("unexpected: {:?}", pair.as_rule()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_typed_params() {
        let p = parse("fn add(db: Db, x: Int) -> Int\n    return x\nend").unwrap();
        match &p.statements[0] {
            Stmt::FnDef { params, return_type, .. } => {
                assert_eq!(params[0].type_ann.as_deref(), Some("Db"));
                assert_eq!(params[1].type_ann.as_deref(), Some("Int"));
                assert_eq!(return_type.as_deref(), Some("Int"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_match_strings() {
        let p = parse("match cmd\n    \"list\" => print(\"ok\")\n    _ => print(\"no\")\nend").unwrap();
        match &p.statements[0] {
            Stmt::Match { arms, .. } => {
                assert!(matches!(&arms[0].pattern, MatchPattern::StringLit(s) if s == "list"));
                assert!(matches!(&arms[1].pattern, MatchPattern::Wildcard));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_todo_parses() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/todo_cli.u")).unwrap();
        let p = parse(&src).unwrap();
        assert!(p.statements.len() > 5);
    }
}
