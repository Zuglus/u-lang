// U Language Transpiler
// Phase 1: U syntax → Rust code → rustc compiles
//
// Architecture:
//   1. Parse .u file → AST
//   2. Generate Rust code from AST
//   3. Call cargo/rustc to compile
//   4. Map errors back to .u source

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
            println!("TODO: build {}", file.display());
            // 1. Read .u file
            // 2. Parse → AST
            // 3. Generate Rust code
            // 4. Write to temp dir
            // 5. Call cargo build
        }
        Cli::Run { file } => {
            println!("TODO: run {}", file.display());
            // Same as build, then execute
        }
        Cli::Check { file } => {
            println!("TODO: check {}", file.display());
            // Parse only, report errors
        }
    }

    Ok(())
}
