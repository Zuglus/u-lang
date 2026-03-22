use clap::Parser;
use std::path::PathBuf;

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
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Build { file } => {
            let bin = compile(&file)?;
            eprintln!("Built: {}", bin.display());
        }
        Cli::Run { file, args } => {
            let bin = compile(&file)?;
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
    }

    Ok(())
}

fn parse_file(path: &PathBuf) -> anyhow::Result<u::ast::Program> {
    let source = std::fs::read_to_string(path)?;
    u::parser::parse(&source)
}

fn compile(path: &PathBuf) -> anyhow::Result<PathBuf> {
    let source = std::fs::read_to_string(path)?;
    let ast = u::parser::parse(&source)?;
    let u_filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("input.u");

    // Find .rs files in the same directory as the .u file
    let u_dir = path.parent().unwrap_or(std::path::Path::new("."));
    let mut rs_modules: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(u_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    rs_modules.push(stem.to_string());
                }
            }
        }
    }
    rs_modules.sort();

    let rust_code = u::generator::generate(&ast, &source, u_filename, &rs_modules)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // Persistent cargo project per .u file — cached builds
    let project_dir = std::env::temp_dir().join("u-lang").join(stem);
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // Runtime path relative to transpiler at compile time
    let runtime_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../runtime"));
    let runtime_path = runtime_dir.canonicalize()
        .map_err(|e| anyhow::anyhow!("runtime crate not found at {}: {}", runtime_dir.display(), e))?;

    // Write Cargo.toml (only if changed)
    let cargo_toml = format!(
r#"[package]
name = "{stem}"
version = "0.1.0"
edition = "2021"

[dependencies]
u-runtime = {{ path = "{}" }}
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

    // Copy .rs module files to generated project
    for module in &rs_modules {
        let src_file = u_dir.join(format!("{}.rs", module));
        let dst_file = src_dir.join(format!("{}.rs", module));
        std::fs::copy(&src_file, &dst_file)?;
    }

    // Write generated source
    std::fs::write(src_dir.join("main.rs"), &rust_code)?;

    // Build with cargo
    let output = std::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--quiet")
        .current_dir(&project_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mapped = remap_errors(&stderr, &rust_code, u_filename);
        anyhow::bail!("build error:\n{}", mapped);
    }

    Ok(project_dir.join("target/release").join(stem))
}

fn remap_errors(stderr: &str, rust_code: &str, u_filename: &str) -> String {
    // Build mapping: generated_line → closest u source line
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

    let mut result = String::new();
    for line in stderr.lines() {
        if let Some(pos) = line.find("src/main.rs:") {
            let after = &line[pos + "src/main.rs:".len()..];
            let num_end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
            if num_end > 0 {
                if let Ok(gen_line) = after[..num_end].parse::<usize>() {
                    let u_line = if gen_line > 0 && gen_line <= line_map.len() {
                        line_map[gen_line - 1]
                    } else { 0 };
                    if u_line > 0 {
                        result.push_str(&line[..pos]);
                        result.push_str(u_filename);
                        result.push(':');
                        result.push_str(&u_line.to_string());
                        // Skip original line:col
                        let mut skip = num_end;
                        if after.as_bytes().get(skip) == Some(&b':') {
                            skip += 1;
                            skip += after[skip..].find(|c: char| !c.is_ascii_digit()).unwrap_or(after[skip..].len());
                        }
                        result.push_str(&after[skip..]);
                        result.push('\n');
                        continue;
                    }
                }
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}
