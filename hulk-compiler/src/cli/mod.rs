// =============================================================================
// CLI (Command Line Interface)
// =============================================================================
//
// Este módulo se encarga de manejar la interfaz de línea de comandos del
// compilador. Sus responsabilidades incluyen:
//
// - Parsear los argumentos de línea de comandos (flags, opciones, archivos)
// - Validar las opciones proporcionadas por el usuario
// - Configurar el compilador según las opciones recibidas
// - Mostrar mensajes de ayuda y versión
// - Manejar errores de entrada del usuario
//
// Ejemplos de flags típicos:
//   --help, -h          Muestra la ayuda
//   --version, -v       Muestra la versión
//   --output, -o        Especifica el archivo de salida
//   --optimize, -O      Nivel de optimización
//   --verbose           Modo verboso para debugging
//
// =============================================================================

use std::env;
use std::fs;
use std::path::PathBuf;

use crate::pipeline::PipelineStage;
use crate::utils::errors::{CompilationError, CompileResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationMode {
    Lex,
    Parse,
    Semantic,
    Ir,
    Codegen,
    Run,
}

impl CompilationMode {
    pub fn as_stage(self) -> PipelineStage {
        match self {
            CompilationMode::Lex => PipelineStage::Lex,
            CompilationMode::Parse => PipelineStage::Parse,
            CompilationMode::Semantic => PipelineStage::Semantic,
            CompilationMode::Ir => PipelineStage::Ir,
            CompilationMode::Codegen => PipelineStage::Codegen,
            CompilationMode::Run => PipelineStage::Codegen,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            CompilationMode::Lex => "lex",
            CompilationMode::Parse => "parse",
            CompilationMode::Semantic => "semantic",
            CompilationMode::Ir => "ir",
            CompilationMode::Codegen => "codegen",
            CompilationMode::Run => "run",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputSource {
    File(PathBuf),
    Inline(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub input: InputSource,
    pub mode: CompilationMode,
    pub output: Option<PathBuf>,
}

impl CliConfig {
    pub fn load_source(&self) -> CompileResult<(String, String)> {
        match &self.input {
            InputSource::File(path) => {
                let source = fs::read_to_string(path).map_err(|error| {
                    CompilationError::io("leer", path.display().to_string(), error.to_string())
                })?;

                Ok((source, path.display().to_string()))
            }
            InputSource::Inline(source) => Ok((source.clone(), "<inline>".to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Help,
    Run(CliConfig),
}

pub fn usage() -> &'static str {
    "Hulk Compiler\n\nUso:\n  hulk-compiler [--lex|--parse|--semantic|--ir|--codegen|--run] [--output <archivo>] <archivo>\n  hulk-compiler --source \"codigo fuente\" [--lex|--parse|--semantic|--ir|--codegen|--run] [--output <archivo>]\n\nOpciones:\n  --lex         Ejecuta solo lexer\n  --parse       Ejecuta lexer + parser\n  --semantic    Ejecuta lexer + parser + semantica\n  --ir          Ejecuta lexer + parser + semantica + IR\n  --codegen     Ejecuta lexer + parser + semantica + IR + codegen LLVM\n  --run         Ejecuta lexer + parser + semantica + IR + codegen LLVM + ensamblado + link + ejecucion\n  --output, -o  Archivo de salida para --codegen; para --run es la ruta del ejecutable final\n  --source      Usa codigo inline en lugar de archivo\n  -h, --help    Muestra esta ayuda"
}

impl CliCommand {
    pub fn from_env() -> CompileResult<Self> {
        Self::parse_args(env::args().skip(1))
    }

    pub fn parse_args<I>(args: I) -> CompileResult<Self>
    where
        I: IntoIterator<Item = String>,
    {
        let mut mode = CompilationMode::Semantic;
        let mut input: Option<InputSource> = None;
        let mut output: Option<PathBuf> = None;

        let mut iterator = args.into_iter();

        while let Some(arg) = iterator.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(CliCommand::Help),
                "--lex" => mode = CompilationMode::Lex,
                "--parse" => mode = CompilationMode::Parse,
                "--semantic" => mode = CompilationMode::Semantic,
                "--ir" => mode = CompilationMode::Ir,
                "--codegen" => mode = CompilationMode::Codegen,
                "--run" => mode = CompilationMode::Run,
                "-o" | "--output" => {
                    let path = iterator.next().ok_or_else(|| {
                        CompilationError::internal("falta la ruta después de --output")
                    })?;
                    output = Some(PathBuf::from(path));
                }
                "--source" => {
                    let source = iterator.next().ok_or_else(|| {
                        CompilationError::internal("falta el texto después de --source")
                    })?;
                    input = Some(InputSource::Inline(source));
                }
                _ if arg.starts_with('-') => {
                    return Err(CompilationError::internal(format!(
                        "opción desconocida: {}",
                        arg
                    )));
                }
                _ => {
                    input = Some(InputSource::File(PathBuf::from(arg)));
                }
            }
        }

        let input = input
            .ok_or_else(|| CompilationError::internal("debes pasar un archivo o usar --source"))?;

        Ok(CliCommand::Run(CliConfig {
            input,
            mode,
            output,
        }))
    }
}

#[cfg(test)]
mod tests;
