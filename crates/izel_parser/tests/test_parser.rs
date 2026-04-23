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

fn count_token_kind(node: &SyntaxNode, needle: TokenKind) -> usize {
    let mut total = 0;
    for child in &node.children {
        match child {
            SyntaxElement::Node(n) => {
                total += count_token_kind(n, needle);
            }
            SyntaxElement::Token(t) if t.kind == needle => {
                total += 1;
            }
            SyntaxElement::Token(_) => {}
        }
    }
    total
}

fn binary_operator_token(node: &SyntaxNode) -> Option<TokenKind> {
    for child in &node.children {
        if let SyntaxElement::Token(t) = child {
            if matches!(
                t.kind,
                TokenKind::Pipe
                    | TokenKind::QuestionQuestion
                    | TokenKind::Or
                    | TokenKind::And
                    | TokenKind::EqEq
                    | TokenKind::NotEq
                    | TokenKind::Is
                    | TokenKind::Lt
                    | TokenKind::Gt
                    | TokenKind::Le
                    | TokenKind::Ge
                    | TokenKind::Caret
                    | TokenKind::Ampersand
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::As
            ) {
                return Some(t.kind);
            }
        }
    }
    None
}

fn first_binary_operator(node: &SyntaxNode) -> Option<TokenKind> {
    if node.kind == NodeKind::BinaryExpr {
        return binary_operator_token(node);
    }

    for child in &node.children {
        if let SyntaxElement::Node(n) = child {
            if let Some(kind) = first_binary_operator(n) {
                return Some(kind);
            }
        }
    }

    None
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
fn parses_plain_assignment_statement_inside_block() {
    let source = r#"
forge main() {
    let x = 0
    x = x + 1
    give x
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::AssignStmt));
}

#[test]
fn parser_precedence_pipeline_vs_sum_keeps_pipeline_at_top_level() {
    let source = "forge main() { let x = a |> b + c }";
    let root = parse_source(source);
    assert_eq!(first_binary_operator(&root), Some(TokenKind::Pipe));
}

#[test]
fn parser_precedence_cast_vs_sum_keeps_sum_at_top_level() {
    let source = "forge main() { let x = y as i32 + z }";
    let root = parse_source(source);
    assert_eq!(first_binary_operator(&root), Some(TokenKind::Plus));
}

#[test]
fn parser_precedence_not_vs_and_keeps_and_at_top_level() {
    let source = "forge main() { let x = not y and z }";
    let root = parse_source(source);
    assert_eq!(first_binary_operator(&root), Some(TokenKind::And));
}

#[test]
fn parses_forge_with_default_parameters_and_variadics() {
    let source = r#"
forge pack(prefix: str = ">", ..args: str) {
    give
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
    assert!(count_kind(&root, NodeKind::ParamPart) >= 2);
    assert!(count_token_kind(&root, TokenKind::Equal) >= 1);
    assert!(count_token_kind(&root, TokenKind::DotDot) >= 1);
}

#[test]
fn parses_shape_with_mixed_field_visibilities() {
    let source = r#"
shape User {
    open id: i32,
    hidden secret: i32,
    pkg shared: i32,
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ShapeDecl));
    assert!(count_kind(&root, NodeKind::Field) >= 3);
    assert!(count_token_kind(&root, TokenKind::Open) >= 1);
    assert!(count_token_kind(&root, TokenKind::Hidden) >= 1);
    assert!(count_token_kind(&root, TokenKind::Pkg) >= 1);
}

#[test]
fn parses_scroll_with_unit_tuple_and_struct_variants() {
    let source = r#"
scroll Event {
    Start,
    Data(i32, str),
    Meta { code: i32 },
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ScrollDecl));
    assert!(count_kind(&root, NodeKind::Variant) >= 3);
}

#[test]
fn parses_branch_with_complex_guard_expression() {
    let source = r#"
forge main() {
    let result = branch x {
        v given v > 0 and v < 10 => v,
        _ => 0,
    }
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::BranchExpr));
    assert!(count_token_kind(&root, TokenKind::Given) >= 1);
    assert!(count_token_kind(&root, TokenKind::FatArrow) >= 2);
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

#[test]
fn recovers_from_missing_closing_brace_in_forge_body() {
    let source = r#"
forge main() {
    let x = 1
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
    assert!(count_kind(&root, NodeKind::LetStmt) >= 1);
}

#[test]
fn recovers_after_invalid_token_in_draw_path_and_parses_following_decl() {
    let source = r#"
draw std::%oops;
forge main() { give 0 }
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::DrawDecl));
    assert!(contains_kind(&root, NodeKind::ForgeDecl));
}

#[test]
fn parses_additional_top_level_decl_forms() {
    let source = r#"
type MaybeI32 = ?i32
draw std/io::*;
static ~COUNTER: i32 = 1
echo { let x = 1 }
bridge "C" { forge puts(msg: str) static errno: i32 }
ward Core { forge helper() { give 0 } }
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::TypeAlias));
    assert!(contains_kind(&root, NodeKind::DrawDecl));
    assert!(contains_kind(&root, NodeKind::StaticDecl));
    assert!(contains_kind(&root, NodeKind::EchoDecl));
    assert!(contains_kind(&root, NodeKind::BridgeDecl));
    assert!(contains_kind(&root, NodeKind::WardDecl));
}

#[test]
fn parses_weave_impl_and_dual_shape_forms() {
    let source = r#"
weave Renderable: Base + Debug { forge render(self) }
weave Renderable for Widget { forge render(self) { give } }
dual shape Codec<T> { value: T, forge encode(self) { give 0 } }
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::WeaveDecl));
    assert!(contains_kind(&root, NodeKind::ImplBlock));
    assert!(contains_kind(&root, NodeKind::DualDecl));
    assert!(contains_kind(&root, NodeKind::Field));
    assert!(count_kind(&root, NodeKind::ForgeDecl) >= 2);
}

#[test]
fn parses_additional_expression_forms_in_forge_body() {
    let source = r#"
forge main() {
    let x = obj?.field
    let y = Type::<i32>{ value: 1 }
    let z = bind |a| a + 1
    let c = seek { 1 } catch err { 0 }
    let w = zone arena { each item in items { while cond { loop { 0 } } } }
    let m = foo.bar(1)
    let p = val! or "msg"
}
"#;

    let root = parse_source(source);
    assert!(contains_kind(&root, NodeKind::MemberExpr));
    assert!(contains_kind(&root, NodeKind::StructLiteral));
    assert!(contains_kind(&root, NodeKind::BindExpr));
    assert!(contains_kind(&root, NodeKind::SeekExpr));
    assert!(contains_kind(&root, NodeKind::ZoneExpr));
    assert!(contains_kind(&root, NodeKind::EachExpr));
    assert!(contains_kind(&root, NodeKind::WhileExpr));
    assert!(contains_kind(&root, NodeKind::LoopExpr));
    assert!(contains_kind(&root, NodeKind::CallExpr));
    assert!(contains_kind(&root, NodeKind::CascadeExpr));
    assert!(count_token_kind(&root, TokenKind::Catch) >= 1);
}
