// =============================================================================
// Tests Unitarios - Pipeline
// =============================================================================
//
// Tests para el módulo Pipeline. Aquí se prueban:
//
// - Flujo correcto entre fases
// - Manejo de errores en cada fase
// - Configuraciones de pipeline parciales
// - Integración entre módulos adyacentes
//
// =============================================================================

#[cfg(test)]
mod tests_pipeline {
    use crate::{CompilationPipeline, PipelineStage};

    #[test]
    fn runs_lex_stage() {
        let pipeline = CompilationPipeline::new("1 + 2", "<test>");
        let report = pipeline.run_to(PipelineStage::Lex).unwrap();

        assert_eq!(report.stage, PipelineStage::Lex);
        assert!(!report.tokens.is_empty());
        assert!(report.program.is_none());
    }

    #[test]
    fn runs_parse_stage() {
        let pipeline = CompilationPipeline::new("1 + 2", "<test>");
        let report = pipeline.run_to(PipelineStage::Parse).unwrap();

        assert_eq!(report.stage, PipelineStage::Parse);
        assert!(report.program.is_some());
    }

    #[test]
    fn runs_semantic_stage() {
        let pipeline = CompilationPipeline::new("1 + 2", "<test>");
        let report = pipeline.run_to(PipelineStage::Semantic).unwrap();

        assert_eq!(report.stage, PipelineStage::Semantic);
        assert!(report.program.is_some());
    }
}
