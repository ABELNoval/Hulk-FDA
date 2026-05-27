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
}

impl PipelineStage {
    pub fn label(self) -> &'static str {
        match self {
            PipelineStage::Lex => "lex",
            PipelineStage::Parse => "parse",
            PipelineStage::Semantic => "semantic",
            PipelineStage::Ir => "ir",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PipelineReport {
    pub stage: PipelineStage,
    pub tokens: Vec<Token>,
    pub program: Option<Program>,
    pub ir: Option<IRModule>,
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
            });
        }

        let program = self.parse(tokens.clone())?;

        if stage == PipelineStage::Parse {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: Some(program),
                ir: None,
            });
        }

        self.semantic(&program)?;

        if stage == PipelineStage::Semantic {
            return Ok(PipelineReport {
                stage,
                tokens,
                program: Some(program),
                ir: None,
            });
        }

        let mut ir = self.ir(&program)?;
        run_ssa_renaming(&mut ir);

        Ok(PipelineReport {
            stage,
            tokens,
            program: Some(program),
            ir: Some(ir),
        })
    }

    pub fn lex(&self) -> CompileResult<Vec<Token>> {
        let mut lexer = Lexer::new(self.source.clone(), self.file_name.clone());
        let (tokens, errors) = lexer.tokenize_with_errors();

        if let Some(diagnostic) = errors.into_iter().next() {
            return Err(CompilationError::from(diagnostic.error));
        }

        Ok(tokens)
    }

    pub fn parse(&self, tokens: Vec<Token>) -> CompileResult<Program> {
        let mut parser = Parser::new(tokens);
        let (program, errors) = parser.parse_program_with_errors();

        if let Some(diagnostic) = errors.into_iter().next() {
            return Err(CompilationError::from(diagnostic.error));
        }

        Ok(program)
    }

    pub fn semantic(&self, program: &Program) -> CompileResult<()> {
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(program).map_err(CompilationError::from)?;
        Ok(())
    }

    pub fn ir(&self, program: &Program) -> CompileResult<IRModule> {
        let mut builder = IRBuilder::new("lowered");
        builder
            .lower_program(program)
            .map_err(|error| CompilationError::internal(error.to_string()))
    }
}

#[cfg(test)]
mod tests;
