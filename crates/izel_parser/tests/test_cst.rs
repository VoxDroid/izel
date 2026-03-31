use izel_lexer::{Token, TokenKind};
use izel_parser::cst::{NodeKind, SyntaxElement, SyntaxNode};
use izel_span::{BytePos, SourceId, Span};

fn sp(lo: u32, hi: u32) -> Span {
    Span::new(BytePos(lo), BytePos(hi), SourceId(0))
}

#[test]
fn syntax_element_span_delegates_for_node_and_token() {
    let token = Token::new(TokenKind::Ident, sp(1, 3));
    let node = SyntaxNode::new(NodeKind::Ident, vec![SyntaxElement::Token(token)]);

    assert_eq!(SyntaxElement::Token(token).span(), sp(1, 3));
    assert_eq!(SyntaxElement::Node(node).span(), sp(1, 3));
}

#[test]
fn syntax_node_span_uses_first_and_last_child_spans() {
    let node = SyntaxNode::new(
        NodeKind::Block,
        vec![
            SyntaxElement::Token(Token::new(TokenKind::OpenBrace, sp(0, 1))),
            SyntaxElement::Token(Token::new(TokenKind::CloseBrace, sp(4, 5))),
        ],
    );

    assert_eq!(node.span(), sp(0, 5));
}

#[test]
#[should_panic(expected = "Span requested for empty SyntaxNode")]
fn syntax_node_span_panics_for_empty_children() {
    let node = SyntaxNode::new(NodeKind::Error, vec![]);
    let _ = node.span();
}
