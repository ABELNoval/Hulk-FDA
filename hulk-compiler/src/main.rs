use std::process;

use hulk_compiler::cli::CliCommand;
use hulk_compiler::execution::{default_codegen_path, default_run_paths, run_native_pipeline};
use hulk_compiler::pipeline::CompilationPipeline;
use hulk_compiler::utils::errors::CompilationError;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error.render_interface());
        process::exit(error.exit_code());
    }
}

fn run() -> hulk_compiler::utils::errors::CompileResult<()> {
    match CliCommand::from_env()? {
        CliCommand::Help => {
            println!("{}", hulk_compiler::cli::usage());
            Ok(())
        }
        CliCommand::Run(config) => {
            let (source, file_name) = config.load_source()?;
            let pipeline = CompilationPipeline::new(source, file_name);
            let report = pipeline.run_to(config.mode.as_stage())?;

            println!("fase ejecutada: {}", config.mode.label());
            println!("tokens generados: {}", report.tokens.len());

            if let Some(program) = report.program {
                println!(
                    "AST listo con {} declaración(es)",
                    program.declarations.len()
                );
            }

            if let Some(ir) = report.ir {
                println!("IR listo con {} función(es)", ir.functions.len());
            }

            if let Some(codegen) = report.codegen {
                if config.mode == hulk_compiler::CompilationMode::Run {
                    let paths = default_run_paths(&config);
                    codegen.write_to_file(&paths.llvm_ir).map_err(|error| {
                        CompilationError::io(
                            "escribir",
                            paths.llvm_ir.display().to_string(),
                            error.to_string(),
                        )
                    })?;

                    println!("LLVM IR escrito en {}", paths.llvm_ir.display());
                    run_native_pipeline(
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
                        &paths.llvm_ir,
                        &paths.bitcode,
                        &paths.executable,
                    )?;
                } else {
                    let output_path = config
                        .output
                        .clone()
                        .unwrap_or_else(|| default_codegen_path(&config));
                    codegen.write_to_file(&output_path).map_err(|error| {
                        CompilationError::io(
                            "escribir",
                            output_path.display().to_string(),
                            error.to_string(),
                        )
                    })?;

                    println!("LLVM IR escrito en {}", output_path.display());
                }
            }

            Ok(())
        }
    }
}
