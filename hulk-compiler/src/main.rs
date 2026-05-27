use std::process;

use hulk_compiler::cli::CliCommand;
use hulk_compiler::pipeline::CompilationPipeline;

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

            Ok(())
        }
    }
}
