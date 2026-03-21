use std::path::PathBuf;

use clap::Parser;
use willow::compile::Compiler;

#[derive(Parser)]
#[command(name = "willow", about = "The willow compiler", version)]
struct Cli {
    /// Path to the entry source file (.wl)
    #[arg(value_parser = validate_input_extension, required_unless_present = "print_targets")]
    input: Option<PathBuf>,

    /// Output executable path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Target triple (e.g. x86_64-unknown-linux-gnu, aarch64-apple-darwin)
    #[arg(long)]
    target: Option<String>,

    /// Optimization level
    #[arg(short = 'O', long, default_value = "0", value_parser = clap::value_parser!(u8).range(0..=3))]
    opt_level: u8,

    /// Emit HIR to stderr
    #[arg(long)]
    emit_hir: bool,

    /// Emit LLVM IR to stderr
    #[arg(long)]
    emit_llvm_ir: bool,

    /// Emit object file only (skip linking)
    #[arg(long)]
    emit_obj: bool,

    /// Print available target architectures and exit
    #[arg(long)]
    print_targets: bool,
}

use inkwell::targets::*;

fn main() {
    let cli = Cli::parse();

    Target::initialize_all(&InitializationConfig::default());

    if cli.print_targets {
        print_available_targets();
        return;
    }

    let input = cli.input.unwrap_or_else(|| {
        eprintln!("error: <INPUT> is required when not using --print-targets");
        std::process::exit(1);
    });

    if let Some(ref triple_str) = cli.target {
        validate_target(triple_str);
    }

    let output = cli.output.unwrap_or_else(|| {
        let stem = input.file_stem().expect("Input file has no name");
        PathBuf::from(stem)
    });

    let options = willow::compile::CompileOptions {
        input,
        output,
        target: cli.target,
        opt_level: cli.opt_level,
        emit_hir: cli.emit_hir,
        emit_llvm_ir: cli.emit_llvm_ir,
        emit_obj: cli.emit_obj,
    };

    let mut compiler = Compiler::default();
    compiler.compile(options);
}

fn validate_input_extension(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    match path.extension().and_then(|e| e.to_str()) {
        Some("wl") => Ok(path),
        _ => Err("entry file must have a .wl extension".to_string()),
    }
}

fn validate_target(triple_str: &str) {
    let triple = TargetTriple::create(triple_str);
    if Target::from_triple(&triple).is_err() {
        eprintln!("error: unknown target '{}'", triple_str);
        eprintln!();
        eprintln!("available targets:");
        print_available_targets();
        std::process::exit(1);
    }
}

fn print_available_targets() {
    let mut target = Target::get_first();
    while let Some(t) = target {
        let name = t.get_name().to_string_lossy();
        let desc = t.get_description().to_string_lossy();
        println!("  {:<20} {}", name, desc);
        target = t.get_next();
    }
}
