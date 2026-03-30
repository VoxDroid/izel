use izel_lexer::{Lexer, TokenKind};
use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_parser::Parser;
use izel_span::SourceId;

fn parse_source(source: &str) -> SyntaxNode {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse_source_file()
}

fn contains_kind(node: &SyntaxNode, needle: NodeKind) -> bool {
    if node.kind == needle {
        return true;
    }

    node.children.iter().any(|child| match child {
        SyntaxElement::Node(n) => contains_kind(n, needle),
        SyntaxElement::Token(_) => false,
    })
}

fn count_kind(node: &SyntaxNode, needle: NodeKind) -> usize {
    let mut total = usize::from(node.kind == needle);
    for child in &node.children {
        if let SyntaxElement::Node(n) = child {
            total += count_kind(n, needle);
        }
    }
    total
}

#[test]
fn parses_core_top_level_declarations() {
    let source = r#"
shape Point { x: i32, y: i32 }
scroll Color { Red, Green, Blue }
forge main() { give 0 }
"#;

    let root = parse_source(source);
    assert_eq!(root.kind, NodeKind::SourceFile);
    assert!(contains_kind(&root, NodeKind::ShapeDecl));
    assert!(contains_kind(&root, NodeKind::ScrollDecl));
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
}

#[test]
fn parses_operator_combinations_inside_function_body() {
    let source = r#"
forge main() {
	let x = a |> b + c
	let y = not x and z
	let z = x as i32 + y
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
    assert!(count_kind(&root, NodeKind::LetStmt) >= 3);
    assert!(count_kind(&root, NodeKind::BinaryExpr) >= 2);
}

#[test]
fn recovers_after_missing_semicolon_sequence() {
    let source = r#"
forge main() {
	let x = 1
	let y = 2
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
    assert!(count_kind(&root, NodeKind::LetStmt) >= 2);
}
