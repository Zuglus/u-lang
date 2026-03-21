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
        Cli::Run { file } => {
            let bin = compile(&file)?;
            let status = std::process::Command::new(&bin).status()?;
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

    let tmp_dir = std::env::temp_dir().join("u-lang");
    std::fs::create_dir_all(&tmp_dir)?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let rs_path = tmp_dir.join(format!("{}.rs", stem));
    let bin_path = tmp_dir.join(stem);

    std::fs::write(&rs_path, &rust_code)?;

    let output = std::process::Command::new("rustc")
        .arg(&rs_path)
        .arg("-o")
        .arg(&bin_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("rustc error:\n{}", stderr);
    }

    Ok(bin_path)
}
