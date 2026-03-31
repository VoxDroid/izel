use izel_doc::DocGenerator;
use izel_parser::ast;
use izel_span::Span;

fn dummy_span() -> Span {
    Span::dummy()
}

fn attr(name: &str, args: Vec<ast::Expr>) -> ast::Attribute {
    ast::Attribute {
        name: name.to_string(),
        args,
        span: dummy_span(),
    }
}

fn forge(name: &str, attributes: Vec<ast::Attribute>) -> ast::Forge {
    ast::Forge {
        name: name.to_string(),
        name_span: dummy_span(),
        visibility: ast::Visibility::Open,
        is_flow: false,
        generic_params: vec![],
        params: vec![],
        ret_type: ast::Type::Prim("void".to_string()),
        effects: vec![],
        attributes,
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block {
            stmts: vec![],
            expr: None,
            span: dummy_span(),
        }),
        span: dummy_span(),
    }
}

fn shape(name: &str) -> ast::Shape {
    ast::Shape {
        name: name.to_string(),
        visibility: ast::Visibility::Open,
        generic_params: vec![],
        fields: vec![],
        attributes: vec![],
        invariants: vec![],
        span: dummy_span(),
    }
}

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

#[test]
fn generate_includes_forge_doc_attributes() {
    let mut generator = DocGenerator::new();
    let module = ast::Module {
        items: vec![ast::Item::Forge(forge(
            "greet",
            vec![attr(
                "doc",
                vec![ast::Expr::Literal(ast::Literal::Str(
                    "Greets the target".to_string(),
                ))],
            )],
        ))],
    };

    let docs = generator.generate(&module);
    assert!(docs.contains("## Forge: greet"));
    assert!(docs.contains("Greets the target"));
}

#[test]
fn generate_includes_shape_headings() {
    let mut generator = DocGenerator::new();
    let module = ast::Module {
        items: vec![ast::Item::Shape(shape("Packet"))],
    };

    let docs = generator.generate(&module);
    assert!(docs.contains("## Shape: Packet"));
}

#[test]
fn generate_ignores_non_doc_and_non_string_doc_attributes() {
    let mut generator = DocGenerator::new();
    let module = ast::Module {
        items: vec![ast::Item::Forge(forge(
            "noisy",
            vec![
                attr("doc", vec![ast::Expr::Literal(ast::Literal::Int(42))]),
                attr(
                    "deprecated",
                    vec![ast::Expr::Literal(ast::Literal::Str("legacy".to_string()))],
                ),
            ],
        ))],
    };

    let docs = generator.generate(&module);
    assert!(docs.contains("## Forge: noisy"));
    assert!(!docs.contains("legacy"));
}

#[test]
fn generate_skips_non_forge_shape_items() {
    let mut generator = DocGenerator::new();
    let module = ast::Module {
        items: vec![ast::Item::Draw(ast::Draw {
            path: vec!["std".to_string(), "io".to_string()],
            is_wildcard: false,
            span: dummy_span(),
        })],
    };

    let docs = generator.generate(&module);
    assert_eq!(docs, "# Module Documentation\n\n");
}

#[test]
fn default_generator_starts_empty_and_renders_header() {
    let mut generator = DocGenerator::default();
    assert!(generator.output.is_empty());

    let docs = generator.generate(&ast::Module { items: vec![] });
    assert_eq!(docs, "# Module Documentation\n\n");
}

#[test]
fn generate_ignores_doc_attribute_without_string_argument() {
    let mut generator = DocGenerator::new();
    let module = ast::Module {
        items: vec![ast::Item::Forge(forge(
            "no_doc_text",
            vec![attr("doc", vec![])],
        ))],
    };

    let docs = generator.generate(&module);
    assert!(docs.contains("## Forge: no_doc_text"));
    assert!(!docs.contains("None"));
}
