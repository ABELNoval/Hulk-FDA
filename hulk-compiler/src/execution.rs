use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::{CliConfig, InputSource};
use crate::utils::errors::{CompilationError, CompileResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeRunPaths {
    pub llvm_ir: PathBuf,
    pub bitcode: PathBuf,
    pub executable: PathBuf,
}

pub fn default_run_paths(config: &CliConfig) -> NativeRunPaths {
    let executable = resolve_path(
        config
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from("output")),
    );

    let llvm_ir = executable.with_extension("ll");
    let bitcode = executable.with_extension("bc");

    NativeRunPaths {
        llvm_ir,
        bitcode,
        executable,
    }
}

pub fn default_codegen_path(config: &CliConfig) -> PathBuf {
    match &config.input {
        InputSource::File(path) => {
            let mut output = path.clone();
            output.set_extension("ll");
            output
        }
        InputSource::Inline(_) => PathBuf::from("output.ll"),
    }
}

pub fn run_native_pipeline(
    project_root: &Path,
    llvm_ir_path: &Path,
    bitcode_path: &Path,
    executable_path: &Path,
) -> CompileResult<()> {
    if let Some(parent) = executable_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            CompilationError::io("crear", parent.display().to_string(), error.to_string())
        })?;
    }

    let command = format!(
        "cd {} && ./scripts/emit_bc_from_ll.sh {} {} && ./scripts/build_from_bc.sh {} {} && {}",
        shell_quote(&shell_path(project_root)),
        shell_quote(&shell_path(llvm_ir_path)),
        shell_quote(&shell_path(bitcode_path)),
        shell_quote(&shell_path(bitcode_path)),
        shell_quote(&shell_path(executable_path)),
        shell_quote(&shell_path(executable_path)),
    );

    let status = shell_command().arg(&command).status().map_err(|error| {
        CompilationError::internal(format!("no se pudo iniciar el flujo nativo: {}", error))
    })?;

    if !status.success() {
        return Err(CompilationError::internal(format!(
            "el flujo nativo terminó con código {:?}",
            status.code()
        )));
    }

    Ok(())
}

fn shell_command() -> Command {
    if cfg!(windows) {
        let mut command = Command::new("wsl");
        command.args(["bash", "-lc"]);
        command
    } else {
        let mut command = Command::new("bash");
        command.args(["-lc"]);
        command
    }
}

fn shell_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");

    if cfg!(windows) && raw.len() >= 2 && raw.as_bytes()[1] == b':' {
        let drive = (raw.as_bytes()[0] as char).to_ascii_lowercase();
        let rest = raw[2..].trim_start_matches('/');
        if rest.is_empty() {
            format!("/mnt/{}", drive)
        } else {
            format!("/mnt/{}/{}", drive, rest)
        }
    } else {
        raw
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn resolve_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}
