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
    /// Enum → варианты
    enums: HashMap<String, Vec<(String, Vec<Type>)>>,
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
        list_methods.insert("first".to_string(), (vec![], Type::Unknown)); // Option[T]
        list_methods.insert("last".to_string(), (vec![], Type::Unknown)); // Option[T]
        list_methods.insert("sum".to_string(), (vec![], Type::Int));
        list_methods.insert("sort".to_string(), (vec![], Type::Unknown)); // List[T]
        list_methods.insert("reverse".to_string(), (vec![], Type::Unknown)); // List[T]
        list_methods.insert("join".to_string(), (vec![Type::String], Type::String));
        ctx.methods.insert("List".to_string(), list_methods);
        
        // Встроенные методы String
        let mut string_methods = HashMap::new();
        string_methods.insert("len".to_string(), (vec![], Type::Int));
        string_methods.insert("trim".to_string(), (vec![], Type::String));
        string_methods.insert("split".to_string(), (vec![Type::String], Type::Unknown)); // List[String]
        string_methods.insert("contains".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("starts_with".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("ends_with".to_string(), (vec![Type::String], Type::Bool));
        string_methods.insert("replace".to_string(), (vec![Type::String, Type::String], Type::String));
        string_methods.insert("to_upper".to_string(), (vec![], Type::String));
        string_methods.insert("to_lower".to_string(), (vec![], Type::String));
        string_methods.insert("to_int".to_string(), (vec![], Type::Unknown)); // Option[Int]
        string_methods.insert("to_float".to_string(), (vec![], Type::Unknown)); // Option[Float]
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
                                    "Несовместимые типы для '{}': {:?} и {:?}. Используйте явное преобразование",
                                    op, left_type, right_type
                                ),
                                span: span.clone(),
                            })
                        }
                        _ => Err(TypeError {
                            message: format!(
                                "Арифметическая операция '{}' не поддерживается для типов {:?} и {:?}",
                                op, left_type, right_type
                            ),
                            span: span.clone(),
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
                        }),
                    }
                }
                
                "&&" | "||" => {
                    // Логические: оба Bool
                    match (&left_type, &right_type) {
                        (Type::Bool, Type::Bool) => Ok(Type::Bool),
                        _ => Err(TypeError {
                            message: format!(
                                "Логическая операция '{}' требует Bool, получено {:?} и {:?}",
                                op, left_type, right_type
                            ),
                            span: span.clone(),
                        }),
                    }
                }
                
                _ => Err(TypeError {
                    message: format!("Неизвестный оператор: {}", op),
                    span: span.clone(),
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
                            });
                        }
                    }
                    
                    Ok(return_type.clone())
                }
                None => Err(TypeError {
                    message: format!("Неизвестная функция: {}", name),
                    span: span.clone(),
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
                    }),
                }
                None => Err(TypeError {
                    message: format!("Нет методов для типа '{}'", type_name),
                    span: span.clone(),
                }),
            }
        }
        
        _ => Ok(Type::Unknown), // Пока не реализовано
    }
}

/// Проверить все инструкции программы
pub fn check_program(program: &Program) -> Result<(), Vec<TypeError>> {
    let mut ctx = TypeCtx::new();
    let mut errors = Vec::new();
    
    // Первый проход: собрать объявления
    for stmt in &program.statements {
        match stmt {
            Stmt::FnDef { name, params, return_type, .. } => {
                let param_types: Vec<Type> = params.iter()
                    .map(|p| parse_type(&p.type_ann.clone().unwrap_or_default()))
                    .collect();
                let ret_type = return_type.as_ref()
                    .map(|t| parse_type(t))
                    .unwrap_or(Type::None);
                ctx.functions.insert(name.clone(), (param_types, ret_type));
            }
            Stmt::StructDef { name, fields, .. } => {
                let field_types: HashMap<String, Type> = fields.iter()
                    .map(|f| (f.name.clone(), parse_type(&f.type_name)))
                    .collect();
                ctx.structs.insert(name.clone(), field_types);
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
        
        Stmt::Return { value, span } => {
            if let Some(expr) = value {
                check_expr(expr, ctx)?;
            }
            Ok(())
        }

        Stmt::Match { expr, arms, span } => {
            let _expr_type = check_expr(expr, ctx)?;
            
            for arm in arms {
                // Проверить паттерн и тело
                ctx.enter_block();
                
                // Добавить переменные из паттерна в контекст
                match &arm.pattern {
                    MatchPattern::Variant { bindings, .. } => {
                        for binding in bindings {
                            ctx.add_var(binding.clone(), Type::Unknown);
                        }
                    }
                    MatchPattern::List(ListPattern::Single(name)) => {
                        ctx.add_var(name.clone(), Type::Unknown);
                    }
                    MatchPattern::List(ListPattern::Cons(head, tail)) => {
                        ctx.add_var(head.clone(), Type::Unknown);
                        ctx.add_var(tail.clone(), Type::Unknown); // List[T]
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
            let iter_type = check_expr(iter, ctx)?;
            
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

fn parse_type(type_name: &str) -> Type {
    match type_name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "String" => Type::String,
        "Bool" => Type::Bool,
        "none" => Type::None,
        s if s.starts_with("List[") && s.ends_with("]") => {
            let inner = &s[5..s.len()-1];
            Type::List(Box::new(parse_type(inner)))
        }
        s => Type::Struct(s.to_string()),
    }
}
