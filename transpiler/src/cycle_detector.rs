use std::collections::{HashMap, HashSet};
use crate::ast::*;

#[derive(Debug)]
pub struct CycleError {
    pub message: String,
    pub span: Span,
}

/// Detects cyclic references in struct definitions
/// Returns Err if a cycle is found, Ok otherwise
pub fn detect_cycles(program: &Program) -> Result<(), CycleError> {
    // Build struct dependency graph
    let mut struct_fields: HashMap<String, Vec<String>> = HashMap::new();
    let mut struct_spans: HashMap<String, Span> = HashMap::new();
    
    for stmt in &program.statements {
        if let Stmt::StructDef { name, fields, span, .. } = stmt {
            let field_types: Vec<String> = fields
                .iter()
                .map(|f| extract_base_type(&f.type_name))
                .filter(|t| is_custom_type(t)) // Only check custom types
                .collect();
            struct_fields.insert(name.clone(), field_types);
            struct_spans.insert(name.clone(), span.clone());
        }
    }
    
    // DFS for each struct to detect cycles
    let mut visited: HashSet<String> = HashSet::new();
    let mut recursion_stack: HashSet<String> = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    
    for struct_name in struct_fields.keys() {
        if !visited.contains(struct_name) {
            if let Some(cycle) = dfs_find_cycle(
                struct_name,
                &struct_fields,
                &mut visited,
                &mut recursion_stack,
                &mut path,
            ) {
                let cycle_str = cycle.join(" -> ");
                let span = struct_spans.get(&cycle[0]).cloned().unwrap_or(Span { start: 0, end: 0 });
                return Err(CycleError {
                    message: format!("Cyclic reference detected: {} -> {}", cycle_str, cycle[0]),
                    span,
                });
            }
        }
    }
    
    Ok(())
}

fn dfs_find_cycle(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    recursion_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    recursion_stack.insert(node.to_string());
    path.push(node.to_string());
    
    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(cycle) = dfs_find_cycle(neighbor, graph, visited, recursion_stack, path) {
                    return Some(cycle);
                }
            } else if recursion_stack.contains(neighbor) {
                // Found cycle - extract cycle from path
                if let Some(pos) = path.iter().position(|x| x == neighbor) {
                    return Some(path[pos..].to_vec());
                }
            }
        }
    }
    
    path.pop();
    recursion_stack.remove(node);
    None
}

/// Extract base type from generic notation like "List[Node]" -> "List"
fn extract_base_type(type_name: &str) -> String {
    type_name.split('[').next().unwrap_or(type_name).to_string()
}

/// Check if type is a custom struct (not primitive)
fn is_custom_type(type_name: &str) -> bool {
    let primitives = [
        "Int", "Float", "String", "Bool", "none",
        "List", "Map", "Result", "Option", "Channel",
    ];
    !primitives.contains(&type_name)
}