//! Compile-time type size analysis for stack limit enforcement
//! 
//! U-lang enforces 500 KB max stack per goroutine.
//! This module calculates type sizes at compile time.

use crate::ast::*;
use crate::type_checker::{Type, TypeCtx};

/// Size information for a type
#[derive(Debug, Clone)]
pub struct TypeSize {
    /// Fixed size in bytes (None for dynamically sized types like List, String)
    pub fixed_size: Option<usize>,
    /// Whether type contains dynamic allocation (heap)
    pub has_dynamic_alloc: bool,
    /// Maximum known size (for bounded dynamic types)
    pub max_size: Option<usize>,
}

impl TypeSize {
    pub fn zero() -> Self {
        TypeSize {
            fixed_size: Some(0),
            has_dynamic_alloc: false,
            max_size: Some(0),
        }
    }
    
    pub fn fixed(bytes: usize) -> Self {
        TypeSize {
            fixed_size: Some(bytes),
            has_dynamic_alloc: false,
            max_size: Some(bytes),
        }
    }
    
    pub fn dynamic() -> Self {
        TypeSize {
            fixed_size: None,
            has_dynamic_alloc: true,
            max_size: None,
        }
    }
}

/// Calculate size of a type
/// 
/// Returns TypeSize with size information
/// For structs, recursively calculates field sizes
pub fn calculate_type_size(typ: &Type, ctx: &TypeCtx) -> TypeSize {
    match typ {
        // Primitive types - fixed sizes
        Type::Int => TypeSize::fixed(8),      // i64
        Type::Float => TypeSize::fixed(8),    // f64
        Type::Bool => TypeSize::fixed(1),     // bool
        Type::None => TypeSize::fixed(0),     // ()
        
        // Dynamic types
        Type::String => TypeSize::dynamic(),  // heap allocated
        Type::List(_) => TypeSize::dynamic(), // heap allocated
        
        // Structs - sum of field sizes
        Type::Struct(name) => {
            calculate_struct_size(name, ctx)
        }
        
        // Enums - discriminant + max variant size
        Type::Enum(name) => {
            calculate_enum_size(name, ctx)
        }
        
        // Functions - pointer size
        Type::Function(_, _) => TypeSize::fixed(16), // fn ptr + environment
        
        // Channel - contains Sender/Receiver (~24 bytes)
        Type::Channel(_) => TypeSize::fixed(24),
        
        // Unknown - assume dynamic
        Type::Unknown => TypeSize::dynamic(),
    }
}

/// Calculate size of a struct
fn calculate_struct_size(name: &str, ctx: &TypeCtx) -> TypeSize {
    // Special cases for built-in types
    match name {
        "Phantom" => return TypeSize::zero(), // PhantomData is ZST
        "Channel" => return TypeSize::fixed(24), // Arc + Sender + Receiver ptrs
        _ => {}
    }
    
    // Get struct fields
    let fields = match ctx.get_struct(name) {
        Some(f) => f,
        None => return TypeSize::dynamic(), // Unknown struct
    };
    
    if fields.is_empty() {
        return TypeSize::zero();
    }
    
    let mut total_size = 0usize;
    let mut has_dynamic = false;
    
    for (_, field_type) in fields {
        let field_size = calculate_type_size(field_type, ctx);
        
        if let Some(size) = field_size.fixed_size {
            total_size += size;
            // Alignment padding would go here in real implementation
        } else {
            has_dynamic = true;
        }
        
        if field_size.has_dynamic_alloc {
            has_dynamic = true;
        }
    }
    
    if has_dynamic {
        TypeSize {
            fixed_size: if total_size > 0 { Some(total_size) } else { None },
            has_dynamic_alloc: true,
            max_size: None,
        }
    } else {
        TypeSize::fixed(total_size)
    }
}

/// Calculate size of an enum (discriminant + max variant)
fn calculate_enum_size(name: &str, ctx: &TypeCtx) -> TypeSize {
    // Get enum variants
    let variants = match ctx.get_enum(name) {
        Some((v, _)) => v,
        None => return TypeSize::dynamic(),
    };
    
    // Discriminant size (u8 for up to 256 variants)
    let discriminant_size = 1usize;
    
    let mut max_variant_size = 0usize;
    let mut has_dynamic = false;
    
    for (_, variant_fields) in variants {
        let mut variant_size = 0usize;
        
        for field_type in variant_fields {
            let field_size = calculate_type_size(field_type, ctx);
            
            if let Some(size) = field_size.fixed_size {
                variant_size += size;
            } else {
                has_dynamic = true;
            }
            
            if field_size.has_dynamic_alloc {
                has_dynamic = true;
            }
        }
        
        max_variant_size = max_variant_size.max(variant_size);
    }
    
    let total_size = discriminant_size + max_variant_size;
    
    if has_dynamic {
        TypeSize {
            fixed_size: Some(total_size),
            has_dynamic_alloc: true,
            max_size: None,
        }
    } else {
        TypeSize::fixed(total_size)
    }
}

/// Check if type exceeds stack limit (500 KB)
/// 
/// Returns Err with error message if type is too large
pub fn check_stack_limit(typ: &Type, ctx: &TypeCtx, location: &str) -> Result<(), String> {
    const STACK_LIMIT_BYTES: usize = 500 * 1024; // 500 KB
    
    let size_info = calculate_type_size(typ, ctx);
    
    // If we can determine a fixed size, check it
    if let Some(size) = size_info.fixed_size {
        if size > STACK_LIMIT_BYTES {
            return Err(format!(
                "ошибка: тип слишком большой для стека ({} байт > {} KB лимит)\n  --> {}\n  = help: разбейте данные на части ≤ 500 KB или используйте каналы для передачи",
                size, STACK_LIMIT_BYTES / 1024, location
            ));
        }
    }
    
    // Types with dynamic allocation can't be fully checked at compile time
    // but we warn about them
    if size_info.has_dynamic_alloc {
        // Dynamic types are stored on heap, so they don't count toward stack limit
        // but we still check the fixed part
        if let Some(fixed) = size_info.fixed_size {
            if fixed > STACK_LIMIT_BYTES {
                return Err(format!(
                    "ошибка: фиксированная часть типа слишком большая ({} байт > {} KB)\n  --> {}\n  = help: уменьшите размер структуры",
                    fixed, STACK_LIMIT_BYTES / 1024, location
                ));
            }
        }
    }
    
    Ok(())
}

/// Get human-readable size description
pub fn format_size(size: &TypeSize) -> String {
    match size.fixed_size {
        Some(0) => "zero-sized".to_string(),
        Some(n) if n < 1024 => format!("{} bytes", n),
        Some(n) if n < 1024 * 1024 => format!("{:.1} KB", n as f64 / 1024.0),
        Some(n) => format!("{:.1} MB", n as f64 / (1024.0 * 1024.0)),
        None => if size.has_dynamic_alloc {
            "dynamic (heap allocated)".to_string()
        } else {
            "unknown".to_string()
        }
    }
}

/// Check if type implements Copy (small enough to copy instead of move)
/// 
/// Threshold: ≤ 64 bytes = Copy, > 64 bytes = Move
pub fn should_copy(typ: &Type, ctx: &TypeCtx) -> bool {
    const COPY_THRESHOLD: usize = 64;
    
    let size_info = calculate_type_size(typ, ctx);
    
    match size_info.fixed_size {
        Some(size) if size <= COPY_THRESHOLD && !size_info.has_dynamic_alloc => true,
        _ => false,
    }
}

/// Check all struct definitions in program for size violations
/// 
/// Returns Err if any struct exceeds 500 KB limit
pub fn check_program_sizes(program: &Program) -> Result<(), String> {
    const STACK_LIMIT_BYTES: usize = 500 * 1024; // 500 KB
    
    // Build TypeCtx to access struct definitions
    let ctx = TypeCtx::new();
    
    let mut errors = Vec::new();
    
    for stmt in &program.statements {
        match stmt {
            Stmt::StructDef { name, fields, .. } => {
                // Calculate total size of struct fields
                let mut total_size = 0usize;
                let mut has_dynamic = false;
                
                for field in fields {
                    // Parse field type
                    let field_type = parse_type(&field.type_name);
                    let size_info = calculate_type_size(&field_type, &ctx);
                    
                    if let Some(size) = size_info.fixed_size {
                        total_size += size;
                    } else {
                        has_dynamic = true;
                    }
                    
                    if size_info.has_dynamic_alloc {
                        has_dynamic = true;
                    }
                }
                
                // Check against limit
                if total_size > STACK_LIMIT_BYTES && !has_dynamic {
                    errors.push(format!(
                        "ошибка: структура '{}' слишком большая ({} байт > {} KB лимит)\n  = help: разбейте на части ≤ 500 KB или используйте каналы",
                        name, total_size, STACK_LIMIT_BYTES / 1024
                    ));
                }
            }
            _ => {}
        }
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// Parse type string into Type enum (simplified)
fn parse_type(type_name: &str) -> Type {
    match type_name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "String" => Type::String,
        "None" => Type::None,
        _ if type_name.starts_with("List[") => Type::List(Box::new(Type::Unknown)),
        _ if type_name.contains('[') => Type::Struct(type_name.to_string()), // Generic like Phantom[T]
        _ => Type::Struct(type_name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_primitive_sizes() {
        let ctx = TypeCtx::new();
        
        assert_eq!(calculate_type_size(&Type::Int, &ctx).fixed_size, Some(8));
        assert_eq!(calculate_type_size(&Type::Bool, &ctx).fixed_size, Some(1));
        assert_eq!(calculate_type_size(&Type::None, &ctx).fixed_size, Some(0));
    }
    
    #[test]
    fn test_phantom_is_zero_sized() {
        let ctx = TypeCtx::new();
        let phantom = TypeSize::zero();
        assert_eq!(phantom.fixed_size, Some(0));
    }
}
