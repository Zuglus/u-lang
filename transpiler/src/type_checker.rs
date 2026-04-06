use crate::ast::*;
use std::collections::HashMap;

/// Тип в U-lang
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    None,
    List(Box<Type>),
    Struct(String),
    Enum(String),
    Function(Vec<Type>, Box<Type>), // параметры, возврат
    Unknown,
}

/// Контекст типизации
pub struct TypeCtx {
    /// Переменные → типы (стек для вложенных блоков)
    vars: Vec<HashMap<String, Type>>,
    /// Функции → сигнатуры
    functions: HashMap<String, (Vec<Type>, Type)>,
    /// Структуры → поля
    structs: HashMap<String, HashMap<String, Type>>,
    /// Enum → (варианты, type_params)
    enums: HashMap<String, (Vec<(String, Vec<Type>)>, Vec<String>)>,
    /// Вариант → имя enum (для быстрого поиска)
    variant_to_enum: HashMap<String, String>,
    /// Методы типов → (имя метода → (параметры, возврат))
    methods: HashMap<String, HashMap<String, (Vec<Type>, Type)>>,
}

impl TypeCtx {
    pub fn new() -> Self {
        let mut ctx = TypeCtx {
            vars: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            variant_to_enum: HashMap::new(),
            methods: HashMap::new(),
        };
        
        // Встроенные функции
        ctx.functions.insert(
            "print".to_string(),
            (vec![Type::String], Type::None),
        );
        
        // Встроенные методы List[T]
        let mut list_methods = HashMap::new();
        list_methods.insert("len".to_string(), (vec![], Type::Int));
        list_methods.insert("first".to_string(), (vec![], Type::Unknown));
        list_methods.insert("last".to_string(), (vec![], Type::Unknown));
        list_methods.insert("sum".to_string(), (vec![], Type::Int));
        list_methods.insert("sort".to_string(), (vec![], Type::Unknown));
        list_methods.insert("reverse".to_string(), (vec![], Type::Unknown));
        list_methods.insert("join".to_string(), (vec![Type::String], Type::String));
        ctx.methods.insert("List".to_string(), list_methods);
        
        // Встроенные методы String
        let mut string_methods = HashMap::new();
        string_methods.insert("len".to_string(), (vec![], Type::Int));
        string_methods.insert("trim".to_string(), (vec![], Type::String));
        string_methods.insert("split".to_string(), (vec![Type::String], Type::Unknown));
        string_methods.insert("contains".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("starts_with".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("ends_with".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("replace".to_string(), (vec![Type::String, Type::String], Type::String));
        string_methods.insert("to_upper".to_string(), (vec![], Type::String));
        string_methods.insert("to_lower".to_string(), (vec![], Type::String));
        string_methods.insert("to_int".to_string(), (vec![], Type::Unknown));
        string_methods.insert("to_float".to_string(), (vec![], Type::Unknown));
        ctx.methods.insert("String".to_string(), string_methods);
        
        // Встроенные методы Int
        let mut int_methods = HashMap::new();
        int_methods.insert("to_string".to_string(), (vec![], Type::String));
        int_methods.insert("abs".to_string(), (vec![], Type::Int));
        ctx.methods.insert("Int".to_string(), int_methods);
        
        // Встроенные методы Float
        let mut float_methods = HashMap::new();
        float_methods.insert("to_string".to_string(), (vec![], Type::String));
        float_methods.insert("abs".to_string(), (vec![], Type::Float));
        ctx.methods.insert("Float".to_string(), float_methods);
        
        ctx
    }

    /// Добавить переменную в текущий блок
    pub fn add_var(&mut self, name: String, typ: Type) {
        if let Some(scope) = self.vars.last_mut() {
            scope.insert(name, typ);
        }
    }

    /// Найти тип переменной
    pub fn get_var(&self, name: &str) -> Option<Type> {
        for scope in self.vars.iter().rev() {
            if let Some(typ) = scope.get(name) {
                return Some(typ.clone());
            }
        }
        None
    }

    /// Войти в новый блок (if, for, fn)
    pub fn enter_block(&mut self) {
        self.vars.push(HashMap::new());
    }

    /// Выйти из блока
    pub fn exit_block(&mut self) {
        self.vars.pop();
    }
}

/// Результат проверки типа
#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub context: Option<String>, // в какой функции/блоке
    pub help: Option<String>,    // подсказка как исправить
}

impl TypeError {
    /// Форматировать ошибку для вывода
    pub fn format(&self, source: &str, filename: &str) -> String {
        let line_num = source[..self.span.start].lines().count() + 1;
        let line_start = source[..self.span.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line_end = source[self.span.start..].find('\n').map(|i| self.span.start + i).unwrap_or(source.len());
        let line = &source[line_start..line_end];
        let col = self.span.start - line_start + 1;
        
        let mut output = format!("\n[ошибка] {}:{}:{}\n", filename, line_num, col);
        output.push_str(&format!("{}\n", self.message));
        
        if let Some(ctx) = &self.context {
            output.push_str(&format!("контекст: {}\n", ctx));
        }
        
        output.push_str(&format!("{:>4} | {}\n", line_num, line));
        
        // Подчеркивание ошибки
        let underline = " ".repeat(col - 1) + &"^".repeat(self.span.end - self.span.start);
        output.push_str(&format!("     | {}\n", underline));
        
        if let Some(help) = &self.help {
            output.push_str(&format!("подсказка: {}\n", help));
        }
        
        output
    }
}

/// Проверить тип выражения
pub fn check_expr(expr: &Expr, ctx: &TypeCtx) -> Result<Type, TypeError> {
    match expr {
        Expr::IntLiteral { .. } => Ok(Type::Int),
        Expr::FloatLiteral { .. } => Ok(Type::Float),
        Expr::StringLiteral { .. } => Ok(Type::String),
        Expr::BoolLiteral { .. } => Ok(Type::Bool),
        Expr::NoneLiteral { .. } => Ok(Type::None),
        
        Expr::Identifier { name, span } => {
            ctx.get_var(name)
                .ok_or_else(|| TypeError {
                    message: format!("Неизвестная переменная: {}", name),
                    span: span.clone(),
                    context: None,
                    help: None,
                })
        }
        
        Expr::BinaryOp { left, op, right, span } => {
            let left_type = check_expr(left, ctx)?;
            let right_type = check_expr(right, ctx)?;
            
            match op.as_str() {
                "+" | "-" | "*" | "/" | "%" => {
                    // Арифметика: только Int или Float
                    match (&left_type, &right_type) {
                        (Type::Int, Type::Int) => Ok(Type::Int),
                        (Type::Float, Type::Float) => Ok(Type::Float),
                        (Type::Int, Type::Float) | (Type::Float, Type::Int) => {
                            Err(TypeError {
                                message: format!(
                                    "Несовместимые типы для '{}': {:?} и {:?}",
                                    op, left_type, right_type
                                ),
                                span: span.clone(),
                                context: None,
                                help: Some(format!(
                                    "Используйте явное преобразование: {}(expr) или {}(expr)",
                                    if left_type == Type::Int { "Float" } else { "Int" },
                                    if right_type == Type::Int { "Float" } else { "Int" }
                                )),
                            })
                        }
                        _ => Err(TypeError {
                            message: format!(
                                "Арифметическая операция '{}' не поддерживается для типов {:?} и {:?}",
                                op, left_type, right_type
                            ),
                            span: span.clone(),
                            context: None,
                            help: None,
                        }),
                    }
                }
                
                "==" | "!=" => {
                    // Сравнение: типы должны совпадать
                    if left_type == right_type {
                        Ok(Type::Bool)
                    } else {
                        Err(TypeError {
                            message: format!(
                                "Нельзя сравнить {:?} и {:?}",
                                left_type, right_type
                            ),
                            span: span.clone(),
                            context: None,
                            help: Some("Операторы == и != требуют одинаковых типов".to_string()),
                        })
                    }
                }
                
                "<" | ">" | "<=" | ">=" => {
                    // Сравнение порядка: Int или Float
                    match (&left_type, &right_type) {
                        (Type::Int, Type::Int) | (Type::Float, Type::Float) => Ok(Type::Bool),
                        _ => Err(TypeError {
                            message: format!(
                                "Сравнение '{}' не поддерживается для {:?} и {:?}",
                                op, left_type, right_type
                            ),
                            span: span.clone(),
                            context: None,
                            help: Some("Используйте операторы < > только для Int или Float".to_string()),
                        }),
                    }
                }
                
                "&&" | "||" | "and" | "or" => {
                    // Логические: оба Bool
                    match (&left_type, &right_type) {
                        (Type::Bool, Type::Bool) => Ok(Type::Bool),
                        _ => Err(TypeError {
                            message: format!(
                                "Логическая операция '{}' требует Bool, получено {:?} и {:?}",
                                op, left_type, right_type
                            ),
                            span: span.clone(),
                            context: None,
                            help: Some("Используйте операторы and/or только для Bool".to_string()),
                        }),
                    }
                }
                
                _ => Err(TypeError {
                    message: format!("Неизвестный оператор: {}", op),
                    span: span.clone(),
                    context: None,
                    help: Some("Доступные операторы: + - * / % == != < > <= >= and or not".to_string()),
                }),
            }
        }
        
        Expr::FunctionCall { name, args, span } => {
            let arg_types: Result<Vec<_>, _> = args.iter()
                .map(|arg| check_expr(arg, ctx))
                .collect();
            let arg_types = arg_types?;
            
            match ctx.functions.get(name) {
                Some((expected_params, return_type)) => {
                    if arg_types.len() != expected_params.len() {
                        return Err(TypeError {
                            message: format!(
                                "Функция '{}' ожидает {} аргументов, получено {}",
                                name, expected_params.len(), arg_types.len()
                            ),
                            span: span.clone(),
                            context: None,
                            help: Some(format!(
                                "Ожидается: {}",
                                expected_params.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join(", ")
                            )),
                        });
                    }
                    
                    for (i, (actual, expected)) in arg_types.iter().zip(expected_params.iter()).enumerate() {
                        if actual != expected {
                            return Err(TypeError {
                                message: format!(
                                    "Аргумент {} функции '{}': ожидался {:?}, получен {:?}",
                                    i + 1, name, expected, actual
                                ),
                                span: span.clone(),
                                context: None,
                                help: Some(format!(
                                    "Аргумент {} должен быть типа {:?}",
                                    i + 1, expected
                                )),
                            });
                        }
                    }
                    
                    Ok(return_type.clone())
                }
                None => Err(TypeError {
                    message: format!("Неизвестная функция: {}", name),
                    span: span.clone(),
                    context: None,
                    help: Some("Проверьте имя функции или импортируйте нужный модуль".to_string()),
                }),
            }
        }

        Expr::MethodCall { object, method, args, span, .. } => {
            let obj_type = check_expr(object, ctx)?;
            let arg_types: Result<Vec<_>, _> = args.iter()
                .map(|arg| check_expr(arg, ctx))
                .collect();
            let arg_types = arg_types?;
            
            // Определяем тип для поиска методов
            let type_name = match &obj_type {
                Type::List(_) => "List",
                Type::String => "String",
                Type::Int => "Int",
                Type::Float => "Float",
                Type::Bool => "Bool",
                Type::Struct(name) => name.as_str(),
                _ => return Err(TypeError {
                    message: format!("Методы не поддерживаются для типа {:?}", obj_type),
                    span: span.clone(),
                    context: None,
                    help: None,
                }),
            };
            
            match ctx.methods.get(type_name) {
                Some(methods) => match methods.get(method) {
                    Some((expected_params, return_type)) => {
                        // +1 для self (объект)
                        if arg_types.len() != expected_params.len() {
                            return Err(TypeError {
                                message: format!(
                                    "Метод '{}.{}' ожидает {} аргументов, получено {}",
                                    type_name, method, expected_params.len(), arg_types.len()
                                ),
                                span: span.clone(),
                                context: None,
                                help: None,
                            });
                        }
                        
                        for (i, (actual, expected)) in arg_types.iter().zip(expected_params.iter()).enumerate() {
                            if actual != expected {
                                return Err(TypeError {
                                    message: format!(
                                        "Аргумент {} метода '{}.{}': ожидался {:?}, получен {:?}",
                                        i + 1, type_name, method, expected, actual
                                    ),
                                    span: span.clone(),
                                    context: None,
                                    help: None,
                                });
                            }
                        }
                        
                        // Для List[T] методы возвращают List[T] или Option[T]
                        if type_name == "List" {
                            match method.as_str() {
                                "first" | "last" => return Ok(Type::Unknown), // Option[T]
                                "sort" | "reverse" => return Ok(obj_type.clone()),
                                _ => {}
                            }
                        }
                        
                        Ok(return_type.clone())
                    }
                    None => Err(TypeError {
                        message: format!("Неизвестный метод '{}.{}'", type_name, method),
                        span: span.clone(),
                        context: None,
                        help: None,
                    }),
                }
                None => Err(TypeError {
                    message: format!("Нет методов для типа '{}'", type_name),
                    span: span.clone(),
                    context: None,
                    help: None,
                }),
            }
        }
        
        Expr::StructInit { name, .. } => {
            // Проверить, является ли это enum вариантом
            if let Some(enum_name) = ctx.variant_to_enum.get(name) {
                return Ok(Type::Enum(enum_name.clone()));
            }
            // Иначе это структура
            Ok(Type::Struct(name.clone()))
        }
        
        _ => Ok(Type::Unknown), // Пока не реализовано
    }
}
pub fn check_program(program: &Program) -> Result<(), Vec<TypeError>> {
    let mut ctx = TypeCtx::new();
    let mut errors = Vec::new();
    
    // Первый проход: собрать объявления
    for stmt in &program.statements {
        match stmt {
            Stmt::FnDef { name, params, return_type, .. } => {
                let param_types: Vec<Type> = params.iter()
                    .map(|p| p.type_ann.as_ref().map(|t| parse_type(t, None)).unwrap_or(Type::Unknown))
                    .collect();
                let ret_type = return_type.as_ref()
                    .map(|t| parse_type(t, None))
                    .unwrap_or(Type::None);
                ctx.functions.insert(name.clone(), (param_types, ret_type));
            }
            Stmt::StructDef { name, fields, .. } => {
                let field_types: HashMap<String, Type> = fields.iter()
                    .map(|f| (f.name.clone(), parse_type(&f.type_name, None)))
                    .collect();
                    .collect();
                ctx.structs.insert(name.clone(), field_types);
            }
            Stmt::TypeDef { name, variants, type_params, .. } => {
                let enum_variants: Vec<(String, Vec<Type>)> = variants.iter()
                    .map(|v| (v.name.clone(), v.fields.iter().map(|f| parse_type(&f.type_name, type_params.as_ref())).collect()))
                    .collect();
                // Заполняем variant_to_enum
                for (vname, _) in &enum_variants {
                    ctx.variant_to_enum.insert(vname.clone(), name.clone());
                }
                let tparams = type_params.clone().unwrap_or_default();
                ctx.enums.insert(name.clone(), (enum_variants, tparams));
            }
            _ => {}
        }
    }
    
    // Второй проход: проверить тела функций
    for stmt in &program.statements {
        if let Err(e) = check_stmt(stmt, &mut ctx) {
            errors.push(e);
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_stmt(stmt: &Stmt, ctx: &mut TypeCtx) -> Result<(), TypeError> {
    match stmt {
        Stmt::Assignment { name, value, .. } => {
            let typ = check_expr(value, ctx)?;
            ctx.add_var(name.clone(), typ);
            Ok(())
        }
        
        Stmt::ExprStmt { expr, .. } => {
            check_expr(expr, ctx)?;
            Ok(())
        }
        
        Stmt::FnDef { name, params, body, .. } => {
            ctx.enter_block();
            
            // Добавить параметры в контекст
            if let Some((param_types, _)) = ctx.functions.get(name).cloned() {
                for (param, typ) in params.iter().zip(param_types.iter()) {
                    ctx.add_var(param.name.clone(), typ.clone());
                }
            }
            
            // Проверить тело
            for stmt in body {
                check_stmt(stmt, ctx)?;
            }
            
            ctx.exit_block();
            Ok(())
        }
        
        Stmt::If { condition, body, elifs, else_body, .. } => {
            let cond_type = check_expr(condition, ctx)?;
            if cond_type != Type::Bool {
                return Err(TypeError {
                    message: format!("Условие if должно быть Bool, получено {:?}", cond_type),
                    span: Span { start: 0, end: 0 },
                    context: None,
                    help: None,
                });
            }
            
            ctx.enter_block();
            for stmt in body {
                check_stmt(stmt, ctx)?;
            }
            ctx.exit_block();
            
            for (cond, block) in elifs {
                let cond_type = check_expr(cond, ctx)?;
                if cond_type != Type::Bool {
                    return Err(TypeError {
                        message: format!("Условие elif должно быть Bool, получено {:?}", cond_type),
                        span: Span { start: 0, end: 0 },
                        context: None,
                        help: None,
                    });
                }
                
                ctx.enter_block();
                for stmt in block {
                    check_stmt(stmt, ctx)?;
                }
                ctx.exit_block();
            }
            
            if let Some(block) = else_body {
                ctx.enter_block();
                for stmt in block {
                    check_stmt(stmt, ctx)?;
                }
                ctx.exit_block();
            }
            
            Ok(())
        }
        
        Stmt::Return { value, span: _ } => {
            if let Some(expr) = value {
                check_expr(expr, ctx)?;
            }
            Ok(())
        }

        Stmt::Match { expr, arms, span: _ } => {
            let expr_type = check_expr(expr, ctx)?;
            eprintln!("DEBUG MATCH: expr_type={:?}, enums={:?}", expr_type, ctx.enums);
            
            for arm in arms {
                ctx.enter_block();
                
                match &arm.pattern {
                    MatchPattern::Variant { name: variant_name, bindings } => {
                        // Получить типы полей для варианта enum
                        let enum_name = match &expr_type {
                            Type::Enum(name) => Some(name.clone()),
                            Type::Struct(name) => Some(name.clone()), // enum может быть распознан как struct
                            _ => None,
                        };
                        
                        eprintln!("DEBUG: expr_type={:?}, enum_name={:?}, enums={:?}", expr_type, enum_name, ctx.enums.keys().collect::<Vec<_>>());
                        
                        let binding_types: Vec<Type> = if let Some(name) = &enum_name {
                            ctx.enums.get(name).and_then(|(variants, _)| {
                                variants.iter().find(|(v, _)| v == variant_name).map(|(_, types)| types.clone())
                            }).unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        
                        eprintln!("DEBUG: variant_name={}, binding_types={:?}", variant_name, binding_types);
                        
                        for (i, binding) in bindings.iter().enumerate() {
                            let typ = binding_types.get(i).cloned().unwrap_or(Type::Unknown);
                            ctx.add_var(binding.clone(), typ);
                        }
                    }
                    MatchPattern::List(ListPattern::Single(name)) => {
                        ctx.add_var(name.clone(), Type::Unknown);
                    }
                    MatchPattern::List(ListPattern::Cons(head, tail)) => {
                        ctx.add_var(head.clone(), Type::Unknown);
                        ctx.add_var(tail.clone(), Type::Unknown);
                    }
                    _ => {}
                }
                
                check_stmt(&arm.body, ctx)?;
                ctx.exit_block();
            }
            
            Ok(())
        }

        Stmt::WhileLoop { condition, body, .. } => {
            let cond_type = check_expr(condition, ctx)?;
            if cond_type != Type::Bool {
                return Err(TypeError {
                    message: format!("Условие while должно быть Bool, получено {:?}", cond_type),
                    span: Span { start: 0, end: 0 },
                    context: None,
                    help: None,
                });
            }
            
            ctx.enter_block();
            for stmt in body {
                check_stmt(stmt, ctx)?;
            }
            ctx.exit_block();
            
            Ok(())
        }

        Stmt::ForLoop { pattern, iter, body, .. } => {
            let _iter_type = check_expr(iter, ctx)?;
            
            ctx.enter_block();
            
            // Добавить переменную цикла
            match pattern {
                ForPattern::Single(name) => {
                    // Если iter это List[T], то переменная типа T
                    ctx.add_var(name.clone(), Type::Unknown);
                }
                ForPattern::Tuple(names) => {
                    for name in names {
                        ctx.add_var(name.clone(), Type::Unknown);
                    }
                }
            }
            
            for stmt in body {
                check_stmt(stmt, ctx)?;
            }
            ctx.exit_block();
            
            Ok(())
        }

        Stmt::Loop { body, .. } => {
            ctx.enter_block();
            for stmt in body {
                check_stmt(stmt, ctx)?;
            }
            ctx.exit_block();
            
            Ok(())
        }

        _ => Ok(()), // Пока не реализовано
    }
}

fn parse_type(type_name: &str, type_params: Option<&Vec<String>>) -> Type {
    match type_name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "String" => Type::String,
        "Bool" => Type::Bool,
        "none" => Type::None,
        s if type_params.map_or(false, |tp| tp.contains(&s.to_string())) => {
            // Type parameter like T in generic enum
            Type::Unknown
        }
        s if s.starts_with("List[") && s.ends_with("]") => {
            let inner = &s[5..s.len()-1];
            Type::List(Box::new(parse_type(inner, type_params)))
        }
        s => Type::Struct(s.to_string()),
    }
}
