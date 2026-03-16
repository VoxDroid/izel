use izel_lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    SourceFile,
    
    // Declarations
    ForgeDecl,
    ShapeDecl,
    ScrollDecl,
    WardDecl,
    DualDecl,
    WeaveDecl,
    ImplBlock,
    DrawDecl,
    GenericParams,
    GenericParam,
    GenericArgs,
    GenericArg,
    Field,
    Variant,
    ParamPart,
    Param,
    ReturnPart,
    Block,
    
    // Statements
    LetStmt,
    ExprStmt,
    
    // Expressions
    Literal,
    Ident,
    BinaryExpr,
    UnaryExpr,
    CallExpr,
    ParenExpr,
    GivenExpr,
    BranchExpr,
    LoopExpr,
    WhileExpr,
    EachExpr,
    OptionalType,
    PointerType,
    BindExpr,
    PathExpr,
    StructLiteral,
    
    // Trivia & Tokens in CST
    Token(TokenKind),
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxElement {
    Node(SyntaxNode),
    Token(izel_lexer::Token),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxNode {
    pub kind: NodeKind,
    pub children: Vec<SyntaxElement>,
}

impl SyntaxElement {
    pub fn span(&self) -> izel_span::Span {
        match self {
            SyntaxElement::Node(node) => node.span(),
            SyntaxElement::Token(token) => token.span,
        }
    }
}

impl SyntaxNode {
    pub fn new(kind: NodeKind, children: Vec<SyntaxElement>) -> Self {
        Self { kind, children }
    }

    pub fn span(&self) -> izel_span::Span {
        if self.children.is_empty() {
            // This should ideally not happen for non-empty source
            panic!("Span requested for empty SyntaxNode: {:?}", self.kind);
        }
        let first = self.children.first().unwrap().span();
        let last = self.children.last().unwrap().span();
        first.to(last)
    }
}
