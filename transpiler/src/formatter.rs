// u fmt — opinionated formatter for .u files.
// One style, no options. Like gofmt.

pub fn format(source: &str) -> String {
    let raw: Vec<&str> = source.lines().collect();
    let joined = join_single_continuations(&raw);
    let split = split_long_chains(&joined);
    let indented = reindent(&split);
    let normalized = normalize_blanks(&indented);

    let mut result = normalized.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

pub fn check(source: &str) -> bool {
    format(source) == source
}

// --- Phase 1: Join SINGLE continuation lines back to previous line ---
// Only joins when there's exactly one continuation (not a multi-line chain).
// This enforces "1 method = 1 line" without destroying intentional multi-line splits.

fn join_single_continuations(lines: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if !trimmed.is_empty() && !is_continuation(trimmed) {
            // Check if next non-blank is a continuation and the one after is NOT
            let next_idx = i + 1;
            if next_idx < lines.len() {
                let next = lines[next_idx].trim();
                let after_is_cont = lines
                    .get(next_idx + 1)
                    .map(|l| is_continuation(l.trim()))
                    .unwrap_or(false);
                if is_continuation(next) && !after_is_cont {
                    // Exactly one continuation — join
                    let mut joined = trimmed.to_string();
                    joined.push_str(next);
                    out.push(joined);
                    i += 2;
                    continue;
                }
            }
        }
        out.push(lines[i].to_string());
        i += 1;
    }
    out
}

fn is_continuation(trimmed: &str) -> bool {
    (trimmed.starts_with('.') && !trimmed.starts_with(".."))
        || trimmed.starts_with("::")
}

// --- Phase 2: Split chains with >2 method calls ---

fn split_long_chains(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            out.push(trimmed.to_string());
            continue;
        }
        let dots = find_chain_positions(trimmed);
        if dots.len() > 2 {
            if dots[0] > 0 {
                out.push(trimmed[..dots[0]].trim_end().to_string());
            }
            for j in 0..dots.len() {
                let start = dots[j];
                let end = if j + 1 < dots.len() { dots[j + 1] } else { trimmed.len() };
                out.push(trimmed[start..end].trim_end().to_string());
            }
        } else {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Find byte positions of .method( and ::method( at parenthesis depth 0
fn find_chain_positions(line: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if in_string {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match bytes[i] {
            b'"' => in_string = true,
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = (depth - 1).max(0),
            b'.' if depth == 0 => {
                if is_method_start(&line[i + 1..]) {
                    positions.push(i);
                }
            }
            b':' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] == b':' => {
                if is_method_start(&line[i + 2..]) {
                    positions.push(i);
                }
            }
            _ => {}
        }
        i += 1;
    }

    positions
}

fn is_method_start(s: &str) -> bool {
    let ident_len = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .count();
    ident_len > 0 && s.len() > ident_len && s.as_bytes()[ident_len] == b'('
}

// --- Phase 3: Reindent based on block structure ---

#[derive(PartialEq, Clone, Copy)]
enum Block {
    Fn,
    For,
    If,
    Loop,
    Match,
    Struct,
    Enum,
    Impl,
    Trait,
}

fn reindent(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut indent: usize = 0;
    let mut stack: Vec<Block> = Vec::new();

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            out.push(String::new());
            continue;
        }

        let is_comment = trimmed.starts_with("//");

        // Dedent before end / elif / else
        if !is_comment && is_dedent_keyword(trimmed) {
            indent = indent.saturating_sub(1);
            if trimmed == "end" {
                stack.pop();
            }
        }

        // Chain continuation lines get +1 indent
        let extra = if is_continuation(trimmed) { 1 } else { 0 };
        out.push(format!("{}{}", "    ".repeat(indent + extra), trimmed));

        // Comments don't affect block tracking
        if is_comment {
            continue;
        }

        // fn inside trait is a signature — no block opened
        let in_trait = stack.last() == Some(&Block::Trait);
        let is_fn_line = is_fn_start(trimmed);

        if opens_block(trimmed) && !(in_trait && is_fn_line) {
            indent += 1;
            stack.push(classify_block(trimmed));
        }

        // elif/else re-open a block (already dedented above)
        if is_mid_block(trimmed) {
            indent += 1;
        }
    }

    out
}

fn is_fn_start(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("pub fn ")
        || trimmed.starts_with("test fn ")
        || trimmed.starts_with("unsafe fn ")
        || trimmed.starts_with("weak fn ")
}

fn classify_block(trimmed: &str) -> Block {
    if is_fn_start(trimmed) {
        Block::Fn
    } else if trimmed.starts_with("for ") {
        Block::For
    } else if trimmed.starts_with("if ") {
        Block::If
    } else if trimmed == "loop" {
        Block::Loop
    } else if trimmed.starts_with("match ") {
        Block::Match
    } else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
        Block::Struct
    } else if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
        Block::Enum
    } else if trimmed.starts_with("impl ") {
        Block::Impl
    } else if trimmed.starts_with("trait ") {
        Block::Trait
    } else {
        Block::Fn
    }
}

fn is_dedent_keyword(trimmed: &str) -> bool {
    trimmed == "end" || trimmed.starts_with("elif ") || trimmed == "else"
}

fn opens_block(trimmed: &str) -> bool {
    is_fn_start(trimmed)
        || trimmed.starts_with("for ")
        || trimmed.starts_with("if ")
        || trimmed == "loop"
        || trimmed.starts_with("match ")
        || trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ")
        || trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("trait ")
}

fn is_mid_block(trimmed: &str) -> bool {
    trimmed.starts_with("elif ") || trimmed == "else"
}

fn is_definition(trimmed: &str) -> bool {
    is_fn_start(trimmed)
        || trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ")
        || trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("trait ")
}

// --- Phase 4: Normalize blank lines ---

fn normalize_blanks(lines: &[String]) -> Vec<String> {
    let mut out = lines.to_vec();

    // Collapse consecutive blank lines (keep max 1)
    let mut i = 1;
    while i < out.len() {
        if out[i].trim().is_empty() && out[i - 1].trim().is_empty() {
            out.remove(i);
        } else {
            i += 1;
        }
    }

    // Insert blank lines where needed
    let mut i = 1;
    while i < out.len() {
        let prev = out[i - 1].trim();
        let curr = out[i].trim();

        if prev.is_empty() || curr.is_empty() {
            i += 1;
            continue;
        }

        let mut need = false;

        // After end: blank if top-level OR next is a definition
        if prev == "end" {
            let top_level = !out[i - 1].starts_with(' ');
            if top_level || is_definition(curr) {
                need = true;
            }
        }

        // Before top-level definition (unless preceded by comment)
        if !out[i].starts_with(' ') && is_definition(curr) && !prev.starts_with("//") {
            need = true;
        }

        // Before // --- separator comment
        if curr.starts_with("// ---") {
            need = true;
        }

        if need {
            out.insert(i, String::new());
            i += 2;
        } else {
            i += 1;
        }
    }

    // Remove leading/trailing blank lines
    while out.first().map_or(false, |l| l.trim().is_empty()) {
        out.remove(0);
    }
    while out.last().map_or(false, |l| l.trim().is_empty()) {
        out.pop();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_indent() {
        assert_eq!(
            format("fn foo()\nprint(\"x\")\nend\n"),
            "fn foo()\n    print(\"x\")\nend\n"
        );
    }

    #[test]
    fn test_nested_indent() {
        assert_eq!(
            format("fn foo()\nif x > 0\nprint(\"yes\")\nend\nend\n"),
            "fn foo()\n    if x > 0\n        print(\"yes\")\n    end\nend\n"
        );
    }

    #[test]
    fn test_trailing_whitespace() {
        assert_eq!(
            format("fn foo()   \n    print(\"x\")  \nend  \n"),
            "fn foo()\n    print(\"x\")\nend\n"
        );
    }

    #[test]
    fn test_chain_split() {
        let out = format("users.filter(fn(x) x > 0).map(fn(x) x.name).join(\", \")\n");
        assert!(out.contains("users\n"));
        assert!(out.contains("    .filter("));
        assert!(out.contains("    .map("));
        assert!(out.contains("    .join("));
    }

    #[test]
    fn test_chain_no_split_two() {
        assert_eq!(
            format("x.filter(fn(a) a > 0).map(fn(a) a.name)\n"),
            "x.filter(fn(a) a > 0).map(fn(a) a.name)\n"
        );
    }

    #[test]
    fn test_chain_join_single() {
        assert_eq!(format("users\n    .len()\n"), "users.len()\n");
    }

    #[test]
    fn test_chain_preserve_multi() {
        // Two continuations — don't join
        let src = "data\n    .group_by(fn(x) x.name)\n    .map_values(fn(x) x.avg())\n";
        let out = format(src);
        assert!(out.contains("data\n"));
        assert!(out.contains("    .group_by("));
        assert!(out.contains("    .map_values("));
    }

    #[test]
    fn test_blank_between_fns() {
        assert_eq!(
            format("fn a()\n    x\nend\nfn b()\n    y\nend\n"),
            "fn a()\n    x\nend\n\nfn b()\n    y\nend\n"
        );
    }

    #[test]
    fn test_idempotent() {
        let src = "fn foo()\n    if x > 0\n        print(\"yes\")\n    end\nend\n";
        assert_eq!(format(src), format(&format(src)));
    }

    #[test]
    fn test_elif_else() {
        assert_eq!(
            format("if a\nx\nelif b\ny\nelse\nz\nend\n"),
            "if a\n    x\nelif b\n    y\nelse\n    z\nend\n"
        );
    }

    #[test]
    fn test_match() {
        assert_eq!(
            format("match cmd\n\"a\" => x\n_ => y\nend\n"),
            "match cmd\n    \"a\" => x\n    _ => y\nend\n"
        );
    }

    #[test]
    fn test_separator_comment() {
        assert_eq!(
            format("x = 1\n// --- section ---\ny = 2\n"),
            "x = 1\n\n// --- section ---\ny = 2\n"
        );
    }

    #[test]
    fn test_file_ends_with_newline() {
        assert_eq!(format("x = 1"), "x = 1\n");
        assert_eq!(format("x = 1\n"), "x = 1\n");
        assert_eq!(format("x = 1\n\n\n"), "x = 1\n");
    }

    #[test]
    fn test_impl_methods_blank_line() {
        let src = "impl Foo\n    fn a()\n        x\n    end\n    fn b()\n        y\n    end\nend\n";
        let out = format(src);
        assert!(out.contains("    end\n\n    fn b()"));
    }

    #[test]
    fn test_comment_before_fn_no_extra_blank() {
        let src = "// doc\nfn foo()\n    x\nend\n";
        assert_eq!(format(src), "// doc\nfn foo()\n    x\nend\n");
    }

    #[test]
    fn test_struct_enum_blank() {
        assert_eq!(
            format("struct A\n    x: Int\nend\nenum B\n    V(x: Int)\nend\n"),
            "struct A\n    x: Int\nend\n\nenum B\n    V(x: Int)\nend\n"
        );
    }

    #[test]
    fn test_trait_method_sig_no_block() {
        let src = "trait Foo\n    fn bar(self) -> Int\nend\n";
        assert_eq!(format(src), "trait Foo\n    fn bar(self) -> Int\nend\n");
    }

    #[test]
    fn test_trait_then_impl() {
        let src = concat!(
            "trait Describable\n",
            "    fn describe(self) -> String\n",
            "end\n",
            "impl Describable for Counter\n",
            "    fn describe(self) -> String\n",
            "        return \"Counter\"\n",
            "    end\n",
            "end\n",
        );
        let out = format(src);
        // trait's fn is a signature — no extra indentation
        assert!(out.contains("trait Describable\n    fn describe(self) -> String\nend"));
        // impl's fn is a definition — has body
        assert!(out.contains("    fn describe(self) -> String\n        return \"Counter\"\n    end"));
    }
}
