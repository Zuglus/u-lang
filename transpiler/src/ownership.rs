//! Ownership tracking for move semantics
//! 
//! Tracks which variables have been moved (can't be used anymore)
//! and which are still available.

use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::type_checker::{Type, TypeCtx};
use crate::size_checker::{calculate_type_size, TypeSize};

/// State of a variable
#[derive(Debug, Clone, PartialEq)]
pub enum VarState {
    /// Variable is available for use
    Live,
    /// Variable has been moved (can't use anymore)
    Moved { to: String, at_line: usize },
    /// Variable was borrowed (temporarily unavailable)
    Borrowed,
}

/// Ownership context - tracks variable states across scopes
pub struct OwnershipCtx {
    /// Stack of scopes, each containing variable states
    scopes: Vec<HashMap<String, VarState>>,
    /// Types of variables (for determining Copy vs Move)
    var_types: HashMap<String, Type>,
    /// Types that implement Copy (cached)
    copy_types: HashSet<String>,
}

impl OwnershipCtx {
    pub fn new() -> Self {
        OwnershipCtx {
            scopes: vec![HashMap::new()],
            var_types: HashMap::new(),
            copy_types: HashSet::from([
                "Int".to_string(),
                "Float".to_string(),
                "Bool".to_string(),
                "None".to_string(),
            ]),
        }
    }
    
    /// Register a new variable
    pub fn add_var(&mut self, name: String, typ: Type) {
        self.var_types.insert(name.clone(), typ);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, VarState::Live);
        }
    }
    
    /// Enter a new scope
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    
    /// Exit current scope
    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }
    
    /// Check if variable is available
    pub fn get_state(&self, name: &str) -> Option<VarState> {
        for scope in self.scopes.iter().rev() {
            if let Some(state) = scope.get(name) {
                return Some(state.clone());
            }
        }
        None
    }
    
    /// Mark variable as moved
    pub fn move_var(&mut self, name: &str, to: String, line: usize) -> Result<(), String> {
        // Check if variable exists
        let state = self.get_state(name);
        match state {
            None => return Err(format!("неизвестная переменная: {}", name)),
            Some(VarState::Moved { to: prev_to, at_line }) => {
                return Err(format!(
                    "переменная '{}' уже перемещена в '{}' на строке {}",
                    name, prev_to, at_line
                ));
            }
            Some(VarState::Borrowed) => {
                return Err(format!(
                    "нельзя переместить '{}' — она занята (borrowed)",
                    name
                ));
            }
            Some(VarState::Live) => {}
        }
        
        // Check if type is Copy (small primitive)
        if let Some(typ) = self.var_types.get(name) {
            if is_copy_type(typ, self) {
                // Copy types don't get moved - they get copied
                return Ok(());
            }
        }
        
        // Mark as moved in the current scope (or parent if not in current)
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), VarState::Moved { to, at_line: line });
                return Ok(());
            }
        }
        
        // If not found in any scope, add to current scope as moved
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), VarState::Moved { to, at_line: line });
        }
        
        Ok(())
    }
    
    /// Check if using a variable is valid (not moved)
    pub fn check_use(&self, name: &str, line: usize) -> Result<(), String> {
        match self.get_state(name) {
            None => Err(format!("неизвестная переменная: {}", name)),
            Some(VarState::Moved { to, at_line }) => {
                Err(format!(
                    "ошибка: использование перемещённой переменной '{}'\n  --> строка {}\n  = перемещена в '{}' на строке {}\n  = help: переменная недоступна после move",
                    name, line, to, at_line
                ))
            }
            Some(VarState::Borrowed) => {
                Err(format!(
                    "ошибка: переменная '{}' временно недоступна (borrowed)\n  --> строка {}",
                    name, line
                ))
            }
            Some(VarState::Live) => Ok(()),
        }
    }
    
    /// Get type of variable
    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.var_types.get(name)
    }
}

/// Check if a type implements Copy (can be copied instead of moved)
fn is_copy_type(typ: &Type, ctx: &OwnershipCtx) -> bool {
    match typ {
        Type::Int | Type::Float | Type::Bool | Type::None => true,
        Type::Struct(name) => {
            // Check if it's a known Copy type
            if ctx.copy_types.contains(name) {
                return true;
            }
            // Small structs (≤64 bytes) are Copy
            // This is a simplified check - in real implementation
            // we'd calculate actual size
            false // Conservative: assume Move unless proven Copy
        }
        Type::Enum(name) => {
            // Small enums are Copy
            false // Conservative
        }
        _ => false, // Lists, functions, etc. are Move
    }
}

/// Analyze ownership for an entire program
pub fn analyze_ownership(program: &Program) -> Result<OwnershipCtx, String> {
    let mut ctx = OwnershipCtx::new();
    let mut errors = Vec::new();
    
    for stmt in &program.statements {
        if let Err(e) = analyze_stmt(stmt, &mut ctx, &mut errors) {
            errors.push(e);
        }
    }
    
    if errors.is_empty() {
        Ok(ctx)
    } else {
        Err(errors.join("\n"))
    }
}

/// Analyze a single statement
fn analyze_stmt(
    stmt: &Stmt,
    ctx: &mut OwnershipCtx,
    errors: &mut Vec<String>
) -> Result<(), String> {
    match stmt {
        Stmt::Assignment { name, value, .. } => {
            // Analyze the value expression first
            analyze_expr(value, ctx, errors, 0)?;
            
            // Add variable to scope
            ctx.add_var(name.clone(), Type::Unknown); // Type would come from type checker
            Ok(())
        }
        
        Stmt::ExprStmt { expr, .. } => {
            analyze_expr(expr, ctx, errors, 0)
        }
        
        Stmt::FnDef { params, body, .. } => {
            ctx.enter_scope();
            
            // Add parameters as live variables
            for param in params {
                ctx.add_var(
                    param.name.clone(),
                    Type::Struct(param.type_ann.clone().unwrap_or_default())
                );
            }
            
            // Analyze body
            for stmt in body {
                if let Err(e) = analyze_stmt(stmt, ctx, errors) {
                    errors.push(e);
                }
            }
            
            ctx.exit_scope();
            Ok(())
        }
        
        Stmt::Spawn { expr, .. } => {
            // Spawn moves captured variables
            if let Expr::Lambda { body, .. } = expr {
                let free_vars = collect_free_vars(body);
                for var in free_vars {
                    if let Err(e) = ctx.move_var(&var, "spawn".to_string(), 0) {
                        errors.push(e);
                    }
                }
            }
            Ok(())
        }
        
        _ => Ok(()), // Other statements don't affect ownership
    }
}

/// Analyze an expression for ownership violations
fn analyze_expr(
    expr: &Expr,
    ctx: &mut OwnershipCtx,
    errors: &mut Vec<String>,
    line: usize
) -> Result<(), String> {
    match expr {
        Expr::Identifier { name, .. } => {
            // Check if variable can be used
            if let Err(e) = ctx.check_use(name, line) {
                errors.push(e);
            }
            Ok(())
        }
        
        Expr::FunctionCall { name, args, .. } => {
            // Function calls move their arguments (unless Copy)
            for arg in args {
                analyze_expr(arg, ctx, errors, line)?;
                
                // If argument is an identifier, it's moved
                if let Expr::Identifier { name: var_name, .. } = arg {
                    if let Err(e) = ctx.move_var(var_name, name.clone(), line) {
                        errors.push(e);
                    }
                }
            }
            Ok(())
        }
        
        Expr::MethodCall { object, args, .. } => {
            // Analyze object and arguments
            analyze_expr(object, ctx, errors, line)?;
            for arg in args {
                analyze_expr(arg, ctx, errors, line)?;
            }
            Ok(())
        }
        
        _ => Ok(()), // Other expressions
    }
}

/// Collect free variables in an expression
fn collect_free_vars(expr: &Expr) -> Vec<String> {
    let mut vars = Vec::new();
    match expr {
        Expr::Identifier { name, .. } => vars.push(name.clone()),
        Expr::FunctionCall { args, .. } => {
            for arg in args {
                vars.extend(collect_free_vars(arg));
            }
        }
        Expr::MethodCall { object, args, .. } => {
            vars.extend(collect_free_vars(object));
            for arg in args {
                vars.extend(collect_free_vars(arg));
            }
        }
        _ => {}
    }
    vars
}
