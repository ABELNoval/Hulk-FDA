pub mod cli;
pub mod codegen;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod execution;
pub mod semantic;
pub mod utils;

pub use cli::{CliCommand, CliConfig, CompilationMode, InputSource};
pub use pipeline::{CompilationPipeline, PipelineReport, PipelineStage};
