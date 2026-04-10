use anyhow::anyhow;
use clap::Parser;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "u", about = "U language transpiler")]
enum Cli {
    /// Build a .u file
    Build {
        #[arg()]
        file: PathBuf,
    },
    /// Run a .u file
    Run {
        #[arg()]
        file: PathBuf,
        /// Arguments passed to the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Check a .u file without building
    Check {
        #[arg()]
        file: PathBuf,
    },
    /// Run test functions
    Test {
        #[arg()]
        file: Option<PathBuf>,
    },
    /// Format .u files
    Format {
        #[arg()]
        files: Vec<PathBuf>,
        /// Check formatting without making changes
        #[arg(long)]
        check: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Build { file } => {
            let bin = compile(&file)?;
            eprintln!("Built: {}", bin.display());
        }
        Cli::Run { file, args } => {
            let bin = build_cached(&file)?;
            let status = std::process::Command::new(&bin)
                .args(&args)
                .status()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Cli::Check { file } => {
            let ast = parse_file(&file)?;
            eprintln!("OK: {} statements", ast.statements.len());
            println!("{:#?}", ast);
        }
        Cli::Format { files, check } => {
            let files = if files.is_empty() { find_u_files(".")? } else { files };
            let mut needs_format = false;
            for f in &files {
                let source = std::fs::read_to_string(f)?;
                let formatted = u::formatter::format(&source);
                if source == formatted { continue; }
                if check {
                    eprintln!("{}: needs formatting", f.display());
                    needs_format = true;
                } else {
                    std::fs::write(f, &formatted)?;
                    eprintln!("formatted: {}", f.display());
                }
            }
            if check && needs_format { std::process::exit(1); }
        }
        Cli::Test { file } => {
            let files = match file {
                Some(f) => vec![f],
                None => find_u_files(".")?,
            };
            let mut total_passed = 0u32;
            let mut total_failed = 0u32;
            for f in &files {
                let source = std::fs::read_to_string(f)?;
                let ast = u::parser::parse(&source)?;
                let test_names: Vec<String> = ast.statements.iter().filter_map(|s| {
                    if let u::ast::Stmt::FnDef { name, is_test: true, .. } = s {
                        Some(name.clone())
                    } else {
                        None
                    }
                }).collect();
                if test_names.is_empty() { continue; }
                if files.len() > 1 { eprintln!("--- {} ---", f.display()); }

                let bin = compile_tests(f, &test_names)?;
                let output = std::process::Command::new(&bin).output()?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                print!("{}", stdout);
                if !output.stderr.is_empty() {
                    eprint!("{}", String::from_utf8_lossy(&output.stderr));
                }

                // Parse "N passed, M failed" from output
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 && parts[1] == "passed," && parts[3] == "failed" {
                        total_passed += parts[0].parse::<u32>().unwrap_or(0);
                        total_failed += parts[2].parse::<u32>().unwrap_or(0);
                    }
                }
            }
            if total_passed > 0 || total_failed > 0 {
                println!("\n{} passed, {} failed", total_passed, total_failed);
            }
            if total_failed > 0 { std::process::exit(1); }
        }
    }

    Ok(())
}

// ─── File discovery ─────────────────────────────────────

fn find_u_files(dir: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if let Some(s) = path.to_str() { files.extend(find_u_files(s)?); }
        } else if path.extension().and_then(|e| e.to_str()) == Some("u") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn parse_file(path: &PathBuf) -> anyhow::Result<u::ast::Program> {
    let source = std::fs::read_to_string(path)?;
    let mut program = u::parser::parse(&source)?;
    
    // Load modules from use statements
    let base_dir = path.parent().unwrap_or(Path::new("."));
    load_modules(&mut program, base_dir)?;
    
    Ok(program)
}

fn load_modules(program: &mut u::ast::Program, base_dir: &Path) -> anyhow::Result<()> {
    let mut loaded = Vec::new();
    
    for stmt in &program.statements {
        if let u::ast::Stmt::UseDecl { path: module, imports: items, .. } = stmt {
            // Skip std modules
            if module.starts_with("std.") || module == "std" {
                continue;
            }
            
            // Try to load module file: module.u or module/module.u
            let module_file = base_dir.join(format!("{}.u", module));
            if module_file.exists() {
                let module_source = std::fs::read_to_string(&module_file)?;
                let module_ast = u::parser::parse(&module_source)?;
                
                // Extract requested items (functions, structs, etc.)
                for item in items {
                    if let Some(stmt) = find_item(&module_ast, item) {
                        loaded.push(stmt);
                    }
                }
            }
        }
    }
    
    // Prepend loaded modules to program
    loaded.extend(program.statements.drain(..));
    program.statements = loaded;
    
    Ok(())
}

fn find_item(ast: &u::ast::Program, name: &str) -> Option<u::ast::Stmt> {
    for stmt in &ast.statements {
        match stmt {
            u::ast::Stmt::FnDef { name: n, .. } if n == name => return Some(stmt.clone()),
            u::ast::Stmt::StructDef { name: n, .. } if n == name => return Some(stmt.clone()),
            u::ast::Stmt::TypeDef { name: n, .. } if n == name => return Some(stmt.clone()),
            _ => {}
        }
    }
    None
}

// ─── Caching ────────────────────────────────────────────

fn build_cached(path: &PathBuf) -> anyhow::Result<PathBuf> {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let cache_dir = cache_dir_for(stem);
    let hash_file = cache_dir.join("hash");
    let bin_path = cache_dir.join("target/release").join(stem);

    let source = std::fs::read_to_string(path)?;
    let hash = compute_hash(path, &source)?;

    // Check cache — if hash matches, run cached binary directly
    if bin_path.exists() {
        if let Ok(cached) = std::fs::read_to_string(&hash_file) {
            if cached.trim() == hash {
                return Ok(bin_path);
            }
        }
    }

    // Compile
    let compiled = compile(path)?;

    // Store hash
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::write(&hash_file, &hash)?;

    Ok(compiled)
}

fn compute_hash(path: &PathBuf, source: &str) -> anyhow::Result<String> {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);

    // Also hash .rs and .u files in same directory (and subdirs for .u modules)
    let dir = path.parent().unwrap_or(Path::new("."));
    hash_dir_files(dir, &mut hasher);

    // Hash subdirectories (for submodules like utils/strings.u)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                hash_dir_files(&p, &mut hasher);
            }
        }
    }

    Ok(format!("{:016x}", hasher.finish()))
}

fn hash_dir_files(dir: &Path, hasher: &mut DefaultHasher) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let ext = p.extension().and_then(|e| e.to_str());
                ext == Some("rs") || ext == Some("u")
            })
            .collect();
        files.sort();
        for f in files {
            if let Ok(content) = std::fs::read_to_string(&f) {
                content.hash(hasher);
            }
        }
    }
}

fn cache_dir_for(stem: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".u-cache").join(stem)
}

// ─── Dependency analysis ────────────────────────────────

struct Deps {
    sqlite: bool,
    http: bool,
    json: bool,
}

fn analyze_deps(program: &u::ast::Program) -> Deps {
    let mut deps = Deps { sqlite: false, http: false, json: false };
    for stmt in &program.statements {
        scan_stmt(stmt, &mut deps);
    }
    deps
}

fn scan_stmt(stmt: &u::ast::Stmt, deps: &mut Deps) {
    use u::ast::*;
    match stmt {
        Stmt::Assignment { value, .. } => scan_expr(value, deps),
        Stmt::ExprStmt { expr, .. } => scan_expr(expr, deps),
        Stmt::FnDef { body, .. } => { for s in body { scan_stmt(s, deps); } }
        Stmt::ForLoop { iter, body, .. } => {
            scan_expr(iter, deps);
            for s in body { scan_stmt(s, deps); }
        }
        Stmt::If { condition, body, elifs, else_body, .. } => {
            scan_expr(condition, deps);
            for s in body { scan_stmt(s, deps); }
            for (c, b) in elifs { scan_expr(c, deps); for s in b { scan_stmt(s, deps); } }
            if let Some(eb) = else_body { for s in eb { scan_stmt(s, deps); } }
        }
        Stmt::Return { value: Some(v), .. } => scan_expr(v, deps),
        Stmt::Match { expr, arms, .. } => {
            scan_expr(expr, deps);
            for arm in arms { scan_stmt(&arm.body, deps); }
        }
        Stmt::MutAssign { object, value, .. } => { scan_expr(object, deps); scan_expr(value, deps); }
        Stmt::Spawn { expr, .. } => scan_expr(expr, deps),
        Stmt::Loop { body, .. } | Stmt::WhileLoop { body, .. } => { for s in body { scan_stmt(s, deps); } }
        Stmt::ImplBlock { methods, .. } => { for m in methods { scan_stmt(m, deps); } }
        _ => {}
    }
}

fn scan_expr(expr: &u::ast::Expr, deps: &mut Deps) {
    use u::ast::*;
    match expr {
        Expr::Identifier { name, .. } => check_ident(name, deps),
        Expr::FunctionCall { name, args, .. } => {
            check_ident(name, deps);
            for a in args { scan_expr(a, deps); }
        }
        Expr::MethodCall { object, args, .. } => {
            scan_expr(object, deps);
            for a in args { scan_expr(a, deps); }
        }
        Expr::FieldAccess { object, .. } => scan_expr(object, deps),
        Expr::BinaryOp { left, right, .. } => { scan_expr(left, deps); scan_expr(right, deps); }
        Expr::UnaryOp { expr: e, .. } | Expr::PostfixOp { expr: e, .. } => scan_expr(e, deps),
        Expr::Lambda { body, .. } => scan_expr(body, deps),
        Expr::List { elements, .. } => { for e in elements { scan_expr(e, deps); } }
        Expr::StructInit { name, fields, .. } => {
            check_ident(name, deps);
            for (_, f) in fields { scan_expr(f, deps); }
        }
        Expr::StringLiteral { parts, .. } => {
            for p in parts {
                if let StringPart::Interpolation(e) = p { scan_expr(e, deps); }
            }
        }
        _ => {}
    }
}

fn check_ident(name: &str, deps: &mut Deps) {
    match name {
        "Sqlite" => deps.sqlite = true,
        "HttpServer" | "Router" | "Response" | "serve" => deps.http = true,
        "parse_json" | "to_json" => deps.json = true,
        _ => {}
    }
}

// ─── Compilation ────────────────────────────────────────

/// Transpiled .u module: module name → generated Rust code
struct UModule {
    name: String,
    rust_code: String,
    fn_params: HashMap<String, Vec<u::ast::FnParam>>,
}

fn compile(path: &PathBuf) -> anyhow::Result<PathBuf> {
    let source = std::fs::read_to_string(path)?;
    let ast = u::parser::parse(&source)?;
    
    // Type checking
    if let Err(errors) = u::type_checker::check_program(&ast) {
        let mut output = String::from("Ошибки типизации:");
        for e in errors {
            output.push_str(&e.format(&source, &path.to_string_lossy()));
        }
        return Err(anyhow!(output));
    }
    
    // Size checking - enforce 500 KB stack limit
    if let Err(e) = u::size_checker::check_program_sizes(&ast) {
        return Err(anyhow!(e));
    }
    
    // Ownership analysis - detect use-after-move
    if let Err(e) = u::ownership::analyze_ownership(&ast) {
        return Err(anyhow!("Ошибки владения:\n{}", e));
    }
    
    // Cycle detection - reject cyclic struct references
    if let Err(e) = u::cycle_detector::detect_cycles(&ast) {
        let line = source[..e.span.start].lines().count();
        return Err(anyhow!(
            "{}:{}: {}\n  = help: Use std.linked for doubly-linked lists or store ID instead of reference",
            path.display(), line, e.message
        ));
    }
    
    let u_filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("input.u");
    let u_dir = path.parent().unwrap_or(Path::new("."));
    let rs_modules = find_rs_modules(u_dir);

    // Discover and transpile .u modules
    let u_modules = discover_u_modules(&ast, u_dir, path)?;
    
    // Cycle detection for modules - disabled (modules may have complex structures)
    
    let all_module_names: Vec<String> = rs_modules.iter().cloned()
        .chain(u_modules.iter().map(|m| m.name.clone()))
        .collect();

    // Collect module fn params for cross-module type info
    let mut ext_fn_params = HashMap::new();
    for m in &u_modules { ext_fn_params.extend(m.fn_params.clone()); }

    let ext_async_fns = find_rs_async_fns(u_dir, &rs_modules);
    let rust_code = u::generator::generate(&ast, &source, u_filename, &all_module_names, &ext_fn_params, &ext_async_fns)
        .map_err(|e| anyhow!("{}", e))?;
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let deps = analyze_deps(&ast);
    build_rust_project(stem, &rust_code, u_dir, &rs_modules, &u_modules, &deps, u_filename, Some(&source))
}

fn compile_tests(path: &PathBuf, test_names: &[String]) -> anyhow::Result<PathBuf> {
    let source = std::fs::read_to_string(path)?;
    let ast = u::parser::parse(&source)?;
    
    // Cycle detection - reject cyclic struct references
    if let Err(e) = u::cycle_detector::detect_cycles(&ast) {
        let line = source[..e.span.start].lines().count();
        return Err(anyhow!(
            "{}:{}: {}\n  = help: Use std.linked for doubly-linked lists or store ID instead of reference",
            path.display(), line, e.message
        ));
    }
    
    let u_filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("input.u");
    let u_dir = path.parent().unwrap_or(Path::new("."));
    let rs_modules = find_rs_modules(u_dir);

    let u_modules = discover_u_modules(&ast, u_dir, path)?;
    let all_module_names: Vec<String> = rs_modules.iter().cloned()
        .chain(u_modules.iter().map(|m| m.name.clone()))
        .collect();

    let mut ext_fn_params = HashMap::new();
    for m in &u_modules { ext_fn_params.extend(m.fn_params.clone()); }

    let ext_async_fns = find_rs_async_fns(u_dir, &rs_modules);
    let rust_code = u::generator::generate(&ast, &source, u_filename, &all_module_names, &ext_fn_params, &ext_async_fns)
        .map_err(|e| anyhow!("{}", e))?;

    // Replace #[tokio::main] and everything after with test runner
    let rust_code = fixup_assertions(&rust_code);
    let rust_code = if let Some(pos) = rust_code.find("#[tokio::main]") {
        let mut code = rust_code[..pos].to_string();
        code.push_str(&generate_test_runner(test_names));
        code
    } else {
        rust_code
    };

    let stem = format!("{}_test",
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("test"));
    let deps = analyze_deps(&ast);
    build_rust_project(&stem, &rust_code, u_dir, &rs_modules, &u_modules, &deps, u_filename, Some(&source))
}

/// Find .u modules referenced by `use` statements and transpile them
fn discover_u_modules(ast: &u::ast::Program, u_dir: &Path, main_path: &Path) -> anyhow::Result<Vec<UModule>> {
    let mut modules = Vec::new();
    let main_stem = main_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    for stmt in &ast.statements {
        if let u::ast::Stmt::UseDecl { path, .. } = stmt {
            // Skip std.* imports
            if path.starts_with("std.") || path == "std" { continue; }

            // Convert dot-path to file path: utils.strings → utils/strings.u
            let file_rel = path.replace('.', "/");
            let u_file = u_dir.join(format!("{}.u", file_rel));

            if u_file.exists() {
                let mod_name = path.replace('.', "_");
                // Avoid importing self
                if mod_name == main_stem { continue; }
                // Avoid duplicates
                if modules.iter().any(|m: &UModule| m.name == mod_name) { continue; }

                let mod_source = std::fs::read_to_string(&u_file)?;
                let mod_ast = u::parser::parse(&mod_source)?;
                let mod_filename = u_file.file_name().and_then(|s| s.to_str()).unwrap_or("module.u");

                let rust_code = u::generator::generate_module(&mod_ast, &mod_source, mod_filename)
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                // Extract pub fn params for cross-module type info
                let mut mod_fn_params = HashMap::new();
                for s in &mod_ast.statements {
                    if let u::ast::Stmt::FnDef { name, params, is_pub: true, .. } = s {
                        mod_fn_params.insert(name.clone(), params.clone());
                    }
                }

                modules.push(UModule { name: mod_name, rust_code, fn_params: mod_fn_params });
            }
        }
    }
    Ok(modules)
}

fn find_rs_modules(dir: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    modules.push(stem.to_string());
                }
            }
        }
    }
    modules.sort();
    modules
}

/// Scan .rs module files for `pub async fn` declarations
fn find_rs_async_fns(dir: &Path, modules: &[String]) -> Vec<String> {
    let mut async_fns = Vec::new();
    for m in modules {
        let path = dir.join(format!("{}.rs", m));
        if let Ok(src) = std::fs::read_to_string(&path) {
            for line in src.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("pub async fn ") {
                    if let Some(rest) = trimmed.strip_prefix("pub async fn ") {
                        let name = rest.split('(').next().unwrap_or("").trim();
                        if !name.is_empty() {
                            async_fns.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    async_fns
}

/// Convert U's assert/assert_eq function calls to Rust macros
fn fixup_assertions(code: &str) -> String {
    code.replace("assert_eq(", "assert_eq!(")
        .replace("assert(", "assert!(")
}

fn generate_test_runner(test_names: &[String]) -> String {
    let mut r = String::new();
    r.push_str("fn main() {\n");
    r.push_str("    std::panic::set_hook(Box::new(|_| {}));\n");
    r.push_str("    let mut passed = 0u32;\n");
    r.push_str("    let mut failed = 0u32;\n\n");

    for name in test_names {
        r.push_str(&format!("    print!(\"  {} ... \");\n", name));
        r.push_str(&format!(
            "    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{ {}(); }})) {{\n",
            name
        ));
        r.push_str("        Ok(_) => { println!(\"ok\"); passed += 1; }\n");
        r.push_str("        Err(e) => {\n");
        r.push_str("            let msg = e.downcast_ref::<String>().map(|s| s.as_str())\n");
        r.push_str("                .or_else(|| e.downcast_ref::<&str>().copied())\n");
        r.push_str("                .unwrap_or(\"panic\");\n");
        r.push_str("            println!(\"FAIL: {}\", msg); failed += 1;\n");
        r.push_str("        }\n");
        r.push_str("    }\n\n");
    }

    r.push_str("    println!(\"\\n{} passed, {} failed\", passed, failed);\n");
    r.push_str("    if failed > 0 { std::process::exit(1); }\n");
    r.push_str("}\n");
    r
}

fn build_rust_project(
    stem: &str, rust_code: &str, u_dir: &Path, rs_modules: &[String],
    u_modules: &[UModule], deps: &Deps, u_filename: &str, u_source: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let project_dir = cache_dir_for(stem);
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Runtime path (resolved at transpiler compile time)
    let runtime_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../runtime"));
    let runtime_path = runtime_dir.canonicalize()
        .map_err(|e| anyhow::anyhow!("runtime crate not found at {}: {}", runtime_dir.display(), e))?;

    // Build features list based on AST analysis
    let mut features = Vec::new();
    if deps.sqlite { features.push("\"sqlite\""); }
    if deps.http { features.push("\"http\""); }
    if deps.json { features.push("\"json\""); }

    let features_str = if features.is_empty() {
        String::new()
    } else {
        format!(", features = [{}]", features.join(", "))
    };

    let cargo_toml = format!(
r#"[package]
name = "{stem}"
version = "0.1.0"
edition = "2021"

[dependencies]
u-runtime = {{ path = "{}"{features_str} }}
tokio = {{ version = "1", features = ["full"] }}
"#,
        runtime_path.display()
    );

    let cargo_path = project_dir.join("Cargo.toml");
    let needs_update = std::fs::read_to_string(&cargo_path)
        .map(|old| old != cargo_toml)
        .unwrap_or(true);
    if needs_update {
        std::fs::write(&cargo_path, &cargo_toml)?;
    }

    // Copy .rs module files
    for module in rs_modules {
        std::fs::copy(
            u_dir.join(format!("{}.rs", module)),
            src_dir.join(format!("{}.rs", module)),
        )?;
    }

    // Write transpiled .u module files
    for m in u_modules {
        std::fs::write(src_dir.join(format!("{}.rs", m.name)), &m.rust_code)?;
    }

    // Write generated source
    std::fs::write(src_dir.join("main.rs"), rust_code)?;

    // Build with cargo
    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--quiet")
        .current_dir(&project_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mapped = u::error_mapper::map_errors(&stderr, rust_code, u_filename, u_source);
        anyhow::bail!("build error:\n{}", mapped);
    }

    Ok(project_dir.join("target/release").join(stem))
}

