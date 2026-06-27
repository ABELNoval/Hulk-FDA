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
            if config.mode == hulk_compiler::CompilationMode::Run {
                let paths = default_run_paths(&config);

                let _ = std::fs::remove_file(&paths.llvm_ir);

                let _ = std::fs::remove_file(&paths.bitcode);

                let _ = std::fs::remove_file(&paths.executable);
            }
            let (source, file_name) = config.load_source()?;
            let pipeline = CompilationPipeline::new(source, file_name);
            let report = pipeline.run_to(config.mode.as_stage())?;

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
                }
            }

            Ok(())
        }
    }
}
