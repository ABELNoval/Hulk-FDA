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
        assert!(report.ir.is_none());
    }

    #[test]
    fn runs_ir_stage() {
        let pipeline = CompilationPipeline::new("1 + 2", "<test>");
        let report = pipeline.run_to(PipelineStage::Ir).unwrap();

        assert_eq!(report.stage, PipelineStage::Ir);
        assert!(report.program.is_some());
        assert!(report.ir.is_some());
        let ir = report.ir.expect("IR must be available at IR stage");
        assert!(!ir.functions.is_empty());
        assert!(report.codegen.is_none());
    }

    #[cfg(not(feature = "llvm-verify"))]
    #[test]
    fn codegen_stage_reports_backend_error_without_feature() {
        let pipeline = CompilationPipeline::new("1 + 2", "<test>");
        let result = pipeline.run_to(PipelineStage::Codegen);

        assert!(result.is_err());
        let error = result.err().unwrap();
        assert!(error.to_string().contains("llvm-verify"));
    }
}
