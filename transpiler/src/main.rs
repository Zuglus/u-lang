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
    let ast = parse_file(path)?;
    let rust_code = u::generator::generate(&ast);

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
        anyhow::bail!("build error:\n{}", stderr);
    }

    Ok(project_dir.join("target/release").join(stem))
}
