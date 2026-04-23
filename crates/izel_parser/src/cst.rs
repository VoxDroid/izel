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
    TypeAlias,
    Type,
    DrawDecl,
    MacroDecl,
    Attribute,
    Attributes,
    RawExpr,
    Identifier,
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
    Effect, // Added Effect variant here

    // Statements
    LetStmt,
    AssignStmt,
    GiveStmt,
    ExprStmt,
    Pattern,

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
    ZoneExpr,
    OptionalType,
    PointerType,
    BindExpr,
    PathExpr,
    StructLiteral,
    MemberExpr,
    CascadeExpr,
    MacroCall,
    Arg,
    SeekExpr,
    StaticDecl,
    EchoDecl,
    BridgeDecl,

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
            // Keep span total even for recovery/empty nodes.
            return izel_span::Span::new(
                izel_span::BytePos(0),
                izel_span::BytePos(0),
                izel_span::SourceId(0),
            );
        }
        let first = self.children.first().unwrap().span();
        let last = self.children.last().unwrap().span();
        first.to(last)
    }
}
