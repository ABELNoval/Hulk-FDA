use std::process;

use hulk_compiler::cli::CliCommand;
use hulk_compiler::pipeline::CompilationPipeline;
use hulk_compiler::utils::errors::CompilationError;

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error);
        process::exit(1);
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
                let output_path = config
                    .output
                    .clone()
                    .unwrap_or_else(|| default_output_path(&config));
                codegen.write_to_file(&output_path).map_err(|error| {
                    CompilationError::io(
                        "escribir",
                        output_path.display().to_string(),
                        error.to_string(),
                    )
                })?;

                println!("LLVM IR escrito en {}", output_path.display());
            }

            Ok(())
        }
    }
}

fn default_output_path(config: &hulk_compiler::cli::CliConfig) -> std::path::PathBuf {
    match &config.input {
        hulk_compiler::cli::InputSource::File(path) => {
            let mut output = path.clone();
            output.set_extension("ll");
            output
        }
        hulk_compiler::cli::InputSource::Inline(_) => std::path::PathBuf::from("output.ll"),
    }
}
