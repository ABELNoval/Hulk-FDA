// =============================================================================
// Pipeline (Orquestador de Compilación)
// =============================================================================
//
// Este módulo actúa como el orquestador principal del proceso de compilación.
// Coordina todas las fases del compilador en el orden correcto:
//
// 1. Lectura del código fuente
// 2. Análisis léxico (Lexer) -> Tokens
// 3. Análisis sintáctico (Parser) -> AST
// 4. Análisis semántico -> AST anotado/validado
// 5. Generación de IR -> Representación intermedia
// 6. Optimizaciones (opcional)
// 7. Generación de código -> Código objetivo
//
// Responsabilidades:
// - Manejar el flujo de datos entre fases
// - Propagar errores de manera adecuada
// - Permitir configuración de qué fases ejecutar
// - Proveer hooks para debugging/logging entre fases
// - Manejar múltiples archivos fuente si es necesario
//
// =============================================================================

use crate::codegen::{CodegenArtifact, CodegenBackend, CodegenContext, CodegenTarget, LlvmInkwellBackend};
use crate::ir::{IRBuilder, IRModule, run_ssa_renaming};
use crate::lexer::{Lexer, Token};
use crate::parser::{Parser, Program};
use crate::semantic::SemanticAnalyzer;
use crate::utils::errors::{CompilationError, CompileResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    Lex,
    Parse,
    Semantic,
    Ir,
    Codegen,
}

impl PipelineStage {
    pub fn label(self) -> &'static str {
        match self {
            PipelineStage::Lex => "lex",
            PipelineStage::Parse => "parse",
            PipelineStage::Semantic => "semantic",
            PipelineStage::Ir => "ir",
            PipelineStage::Codegen => "codegen",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineReport {
    pub stage: PipelineStage,
    pub tokens: Vec<Token>,
    pub program: Option<Program>,
    pub ir: Option<IRModule>,
    pub codegen: Option<CodegenArtifact>,
}

#[derive(Debug, Clone)]
pub struct CompilationPipeline {
    source: String,
    file_name: String,
}

impl CompilationPipeline {
    pub fn new(source: impl Into<String>, file_name: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            file_name: file_name.into(),
        }
    }

    pub fn run_to(&self, stage: PipelineStage) -> CompileResult<PipelineReport> {
        let tokens = self.lex()?;

        if stage == PipelineStage::Lex {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: None,
                ir: None,
                codegen: None,
            });
        }

        let program = self.parse(tokens.clone())?;

        if stage == PipelineStage::Parse {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: Some(program),
                ir: None,
                codegen: None,
            });
        }

        self.semantic(&program)?;

        if stage == PipelineStage::Semantic {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: Some(program),
                ir: None,
                codegen: None,
            });
        }

        let mut ir = self.ir(&program)?;
        run_ssa_renaming(&mut ir);

        if stage == PipelineStage::Ir {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: Some(program),
                ir: Some(ir),
                codegen: None,
            });
        }

        let codegen = self.codegen(&ir)?;

        Ok(PipelineReport {
            stage,
            tokens,
            program: Some(program),
            ir: Some(ir),
            codegen: Some(codegen),
        })
    }

    pub fn lex(&self) -> CompileResult<Vec<Token>> {
        let mut lexer = Lexer::new(self.source.clone(), self.file_name.clone());
        let (tokens, errors) = lexer.tokenize_with_errors();

        if let Some(diagnostic) = errors.into_iter().next() {
            return Err(CompilationError::lexer(diagnostic.error, diagnostic.span));
        }

        Ok(tokens)
    }

    pub fn parse(&self, tokens: Vec<Token>) -> CompileResult<Program> {
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse_program_with_errors();

        if let Some(diagnostic) = errors.into_iter().next() {
            return Err(CompilationError::parser(diagnostic.error, diagnostic.span));
        }

        Ok(program)
    }

    pub fn semantic(&self, program: &Program) -> CompileResult<()> {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(program).map_err(CompilationError::semantic)?;
        Ok(())
    }

    pub fn ir(&self, program: &Program) -> CompileResult<IRModule> {
        let mut builder = IRBuilder::new("lowered");
        builder
            .lower_program(program)
            .map_err(|error| CompilationError::internal(error.to_string()))
    }

    pub fn codegen(&self, ir: &IRModule) -> CompileResult<CodegenArtifact> {
        let backend = LlvmInkwellBackend::new();
        let context = CodegenContext::new(self.module_name(), CodegenTarget::LlvmIr);

        backend
            .emit_module(ir, &context)
            .map_err(|error| CompilationError::internal(error.to_string()))
    }

    pub fn module_name(&self) -> String {
        let raw = self.file_name.trim();

        if raw == "<inline>" {
            return "inline".to_string();
        }

        let candidate = std::path::Path::new(raw)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("lowered");

        let sanitized: String = candidate
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
            .collect();

        if sanitized.is_empty() {
            "lowered".to_string()
        } else {
            sanitized
        }
    }
}

#[cfg(test)]
mod tests;
