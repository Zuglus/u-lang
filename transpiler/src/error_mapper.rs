/// Error mapper: rustc stderr → U language error messages
///
/// Builds a line map from `// line:N` comments in generated Rust code,
/// then rewrites rustc error output to point at original .u source lines
/// and translates common Rust errors into U-friendly messages.

use std::collections::HashMap;

/// A single translated error
struct UError {
    level: String,      // "ошибка" / "предупреждение"
    message: String,    // translated message
    u_file: String,     // e.g. "script.u"
    u_line: usize,      // line in .u file
    source_line: String, // the actual .u source line (if available)
    hint: String,        // optional hint
}

/// Build mapping: generated .rs line number → original .u line number
fn build_line_map(rust_code: &str) -> Vec<usize> {
    let mut line_map: Vec<usize> = Vec::new();
    let mut last_u_line = 0;
    for line in rust_code.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("// line:") {
            if let Ok(n) = rest.parse::<usize>() {
                last_u_line = n;
            }
        }
        line_map.push(last_u_line);
    }
    line_map
}

/// Error translation rules: (rustc pattern, U message, optional hint)
fn translation_rules() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Mutability
        (
            "cannot borrow",
            "нельзя изменить: переменная не объявлена как mut",
            "добавь mut в определение функции",
        ),
        (
            "cannot borrow `self` as mutable",
            "нельзя изменить self: метод не объявлен как mut",
            "добавь mut self в параметры метода",
        ),
        (
            "cannot assign to",
            "нельзя изменить: параметр не объявлен как mut",
            "добавь mut перед параметром в fn",
        ),
        (
            "as it is a captured variable in a `Fn` closure",
            "нельзя изменить захваченную переменную в замыкании",
            "используй Channel (.send/.recv) вместо прямой мутации",
        ),
        // Variables and names
        (
            "cannot find value",
            "неизвестная переменная",
            "",
        ),
        (
            "not found in this scope",
            "неизвестное имя",
            "проверь написание или добавь use",
        ),
        (
            "cannot find function",
            "неизвестная функция",
            "проверь написание или добавь use",
        ),
        (
            "cannot find type",
            "неизвестный тип",
            "проверь написание или добавь use",
        ),
        (
            "cannot find struct",
            "неизвестная структура",
            "проверь написание или добавь use",
        ),
        // Types
        (
            "mismatched types",
            "несоответствие типов",
            "",
        ),
        (
            "expected",
            "несоответствие типов",
            "",
        ),
        (
            "the trait bound",
            "тип не поддерживает нужную операцию",
            "",
        ),
        // Ownership / borrowing
        (
            "use of moved value",
            "значение уже использовано (перемещено)",
            "используй .clone() или перестрой логику",
        ),
        (
            "value used here after move",
            "значение уже использовано (перемещено)",
            "используй .clone() или перестрой логику",
        ),
        (
            "does not live long enough",
            "значение не живёт достаточно долго",
            "переменная уничтожается раньше, чем используется",
        ),
        (
            "cannot move out of",
            "нельзя переместить значение",
            "используй .clone() или ссылку &",
        ),
        // Operators
        (
            "binary operation",
            "операция не поддерживается для этих типов",
            "проверь типы операндов",
        ),
        (
            "cannot add",
            "нельзя сложить: разные типы",
            "приведи типы к одному (str(), int())",
        ),
        (
            "cannot apply unary operator",
            "унарная операция не поддерживается для этого типа",
            "",
        ),
        // Index / access
        (
            "cannot index into a value of type",
            "нельзя обратиться по индексу",
            "тип не поддерживает доступ по индексу []",
        ),
        (
            "no field",
            "поле не найдено",
            "проверь имя поля в определении struct",
        ),
        (
            "no method named",
            "метод не найден",
            "проверь имя метода или добавь impl",
        ),
        // Arguments
        (
            "this function takes",
            "неверное количество аргументов",
            "",
        ),
        (
            "unexpected argument",
            "лишний аргумент",
            "",
        ),
        (
            "missing argument",
            "пропущен аргумент",
            "",
        ),
        // Return
        (
            "match arms have incompatible types",
            "ветки match возвращают разные типы",
            "все ветки должны возвращать один тип",
        ),
        (
            "`match` arms have incompatible types",
            "ветки match возвращают разные типы",
            "все ветки должны возвращать один тип",
        ),
        // Spawn-related
        (
            "closure may outlive the current function",
            "нельзя использовать локальную переменную в spawn",
            "используй Channel (.send/.recv) для передачи данных",
        ),
        (
            "cannot be sent between threads safely",
            "этот тип нельзя передать в spawn",
            "используй Channel или Arc для разделяемых данных",
        ),
    ]
}

/// Extract type info from "expected X, found Y" patterns
fn extract_type_mismatch(line: &str) -> Option<(String, String)> {
    // Pattern: "expected `X`, found `Y`"
    if let Some(pos) = line.find("expected `") {
        let rest = &line[pos + 10..];
        if let Some(end) = rest.find('`') {
            let expected = map_rust_type_to_u(&rest[..end]);
            if let Some(pos2) = rest.find("found `") {
                let rest2 = &rest[pos2 + 7..];
                if let Some(end2) = rest2.find('`') {
                    let found = map_rust_type_to_u(&rest2[..end2]);
                    return Some((expected, found));
                }
            }
        }
    }
    None
}

/// Map Rust type names back to U type names
fn map_rust_type_to_u(t: &str) -> String {
    match t {
        "i64" => "Int".into(),
        "f64" => "Float".into(),
        "bool" => "Bool".into(),
        "String" | "&str" | "&String" => "String".into(),
        "()" => "none".into(),
        "Vec<i64>" => "List[Int]".into(),
        "Vec<String>" => "List[String]".into(),
        _ => {
            // Strip references
            let t = t.strip_prefix("&mut ").or(Some(t)).unwrap();
            let t = t.strip_prefix('&').unwrap_or(t);
            t.to_string()
        }
    }
}

/// Translate a single error message
fn translate_message(rust_msg: &str) -> (String, String) {
    let rules = translation_rules();
    for (pattern, u_msg, hint) in &rules {
        if rust_msg.contains(pattern) {
            let mut message = u_msg.to_string();
            // Enrich type mismatch with concrete types
            if *u_msg == "несоответствие типов" {
                if let Some((expected, found)) = extract_type_mismatch(rust_msg) {
                    message = format!("несоответствие типов: ожидался {}, получен {}", expected, found);
                }
            }
            return (message, hint.to_string());
        }
    }
    // No translation found — return cleaned-up original
    (clean_rust_message(rust_msg), String::new())
}

/// Clean up Rust-specific noise from error messages
fn clean_rust_message(msg: &str) -> String {
    msg.replace("i64", "Int")
       .replace("f64", "Float")
       .replace("&str", "String")
       .replace("String", "String")
       .replace("bool", "Bool")
       .replace("Vec<", "List[")
       .replace(">", "]")
       .replace("()", "none")
}

/// Read .u source file and return lines
fn read_source_lines(_u_filename: &str, u_source: Option<&str>) -> HashMap<usize, String> {
    let mut lines = HashMap::new();
    if let Some(source) = u_source {
        for (i, line) in source.lines().enumerate() {
            lines.insert(i + 1, line.to_string());
        }
    }
    lines
}

/// Main entry point: remap and translate rustc errors
pub fn map_errors(stderr: &str, rust_code: &str, u_filename: &str, u_source: Option<&str>) -> String {
    let line_map = build_line_map(rust_code);
    let source_lines = read_source_lines(u_filename, u_source);

    let mut result = String::new();
    let mut lines_iter = stderr.lines().peekable();
    let mut seen_errors: Vec<(usize, String)> = Vec::new();

    while let Some(line) = lines_iter.next() {
        let trimmed = line.trim();

        // Match "error[E0xxx]: message" or "error: message" header lines
        if line.starts_with("error") || line.starts_with("warning") {
            // Skip summary lines
            if trimmed.starts_with("error: could not compile")
                || trimmed.starts_with("error: aborting due to")
                || trimmed.starts_with("error[E0601]") // missing main (internal)
            {
                continue;
            }

            // Extract the error message
            let msg = if let Some(pos) = line.find("]: ") {
                &line[pos + 3..]
            } else if let Some(pos) = line.find(": ") {
                &line[pos + 2..]
            } else {
                continue;
            };

            // Try to get line number from this line or the next "--> " line
            let mut gen_line: Option<usize> = extract_line_number(line);
            if gen_line.is_none() {
                // Peek at next lines for "--> src/main.rs:LINE"
                while let Some(next) = lines_iter.peek() {
                    let nt = next.trim();
                    if nt.starts_with("--> src/main.rs:") {
                        gen_line = extract_line_number(nt);
                        lines_iter.next(); // consume
                        break;
                    } else if nt.starts_with("--> ") || nt.is_empty() || nt.starts_with("error") || nt.starts_with("warning") {
                        break;
                    } else {
                        lines_iter.next(); // skip intermediate lines
                    }
                }
            }

            let u_line = gen_line.map(|gl| lookup_line(&line_map, gl)).unwrap_or(0);
            let level = if line.starts_with("error") { "ошибка" } else { "предупреждение" };
            let (translated, hint) = translate_message(msg);

            let key = (u_line, translated.clone());
            if seen_errors.contains(&key) { continue; }
            seen_errors.push(key);

            let source = source_lines.get(&u_line).cloned().unwrap_or_default();
            let err = UError {
                level: level.to_string(),
                message: translated,
                u_file: u_filename.to_string(),
                u_line,
                source_line: source,
                hint,
            };
            result.push_str(&format_u_error(&err));
            continue;
        }

        // Skip noisy rustc lines
        if trimmed.starts_with("For more information about this error") { continue; }
        if trimmed.starts_with("Some errors have detailed explanations") { continue; }
        if trimmed.starts_with("aborting due to") { continue; }
        if trimmed.starts_with("could not compile") { continue; }
        if trimmed.starts_with("--> src/main.rs:") { continue; } // stray location lines
        if trimmed.is_empty() { continue; }

        // Skip rustc source display lines ("|", "= help:", etc.)
        if trimmed.starts_with('|') || trimmed.starts_with("= help") || trimmed.starts_with("= note") { continue; }
        // Skip line number display lines like "4 | let x = ..."
        if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) && trimmed.contains(" | ") { continue; }
    }

    if result.is_empty() && !stderr.is_empty() {
        return clean_rust_message(stderr);
    }

    result
}

fn lookup_line(line_map: &[usize], gen_line: usize) -> usize {
    if gen_line > 0 && gen_line <= line_map.len() {
        line_map[gen_line - 1]
    } else {
        0
    }
}

fn extract_line_number(line: &str) -> Option<usize> {
    if let Some(pos) = line.find("src/main.rs:") {
        let after = &line[pos + "src/main.rs:".len()..];
        let num_end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
        if num_end > 0 {
            return after[..num_end].parse::<usize>().ok();
        }
    }
    None
}

fn format_u_error(err: &UError) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!("{}: {}\n", err.level, err.message));

    // Location
    if err.u_line > 0 {
        out.push_str(&format!("  --> {}:{}\n", err.u_file, err.u_line));

        // Source line
        if !err.source_line.is_empty() {
            out.push_str("   |\n");
            out.push_str(&format!("{:>3} | {}\n", err.u_line, err.source_line));
            out.push_str("   |\n");
        }
    }

    // Hint
    if !err.hint.is_empty() {
        out.push_str(&format!("   = подсказка: {}\n", err.hint));
    }

    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_map() {
        let code = "fn main() {\n// line:5\nlet x = 1;\n// line:6\nlet y = 2;\n}";
        let map = build_line_map(code);
        assert_eq!(map, vec![0, 5, 5, 6, 6, 6]);
    }

    #[test]
    fn test_translate_borrow() {
        let (msg, hint) = translate_message("cannot borrow `x` as mutable, as it is not declared as mutable");
        assert!(msg.contains("нельзя изменить"));
        assert!(!hint.is_empty());
    }

    #[test]
    fn test_translate_not_found() {
        let (msg, _) = translate_message("cannot find value `foo` in this scope");
        assert_eq!(msg, "неизвестная переменная");
    }

    #[test]
    fn test_translate_type_mismatch() {
        let (msg, _) = translate_message("mismatched types: expected `i64`, found `bool`");
        assert!(msg.contains("ожидался Int, получен Bool"));
    }

    #[test]
    fn test_map_rust_type() {
        assert_eq!(map_rust_type_to_u("i64"), "Int");
        assert_eq!(map_rust_type_to_u("f64"), "Float");
        assert_eq!(map_rust_type_to_u("bool"), "Bool");
        assert_eq!(map_rust_type_to_u("&str"), "String");
        assert_eq!(map_rust_type_to_u("()"), "none");
    }

    #[test]
    fn test_full_remap() {
        let rust_code = "// line:1\nfn main() {\n// line:3\nlet x = 1;\n}\n";
        let stderr = "error[E0425]: cannot find value `foo` in this scope\n --> src/main.rs:4:5\n";
        let result = map_errors(stderr, rust_code, "test.u", None);
        assert!(result.contains("test.u"));
        assert!(result.contains("неизвестная переменная"));
    }
}
