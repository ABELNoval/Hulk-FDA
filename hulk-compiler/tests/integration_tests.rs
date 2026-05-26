use hulk_compiler::pipeline::{CompilationPipeline, PipelineStage};

#[test]
fn end_to_end_pipeline_reaches_semantic() {
    let pipeline = CompilationPipeline::new("1 + 2 * 3", "<integration>");
    let report = pipeline.run_to(PipelineStage::Semantic).unwrap();

    assert_eq!(report.stage, PipelineStage::Semantic);
    assert!(report.tokens.len() > 1);
    assert!(report.program.is_some());
}

#[test]
fn end_to_end_pipeline_reports_lexer_errors() {
    let pipeline = CompilationPipeline::new("?", "<integration>");
    let error = pipeline.run_to(PipelineStage::Lex).unwrap_err();

    assert_eq!(error.phase(), "lexer");
}
