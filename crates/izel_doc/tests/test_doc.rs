use izel_doc::DocGenerator;
use izel_parser::ast;

#[test]
fn generate_emits_header_for_empty_module() {
    let mut generator = DocGenerator::new();
    let module = ast::Module { items: vec![] };

    let docs = generator.generate(&module);
    assert!(docs.starts_with("# Module Documentation"));
}

#[test]
fn generate_resets_previous_output_before_rendering() {
    let mut generator = DocGenerator::new();
    generator.output.push_str("stale content");
    let module = ast::Module { items: vec![] };

    let docs = generator.generate(&module);
    assert!(!docs.contains("stale content"));
    assert_eq!(docs, "# Module Documentation\n\n");
}
