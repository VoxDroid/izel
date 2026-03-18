use izel_span::Span;

#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    Forge(Forge),
    Shape(Shape),
    Scroll(Scroll),
    Weave(Weave),
    Impl(Impl),
    Alias(Alias),
    Ward(Ward),
    Draw(Draw),
}

#[derive(Debug, Clone)]
pub struct Forge {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret_type: Type,
    pub effects: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub body: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Shape {
    pub name: String,
    pub generic_params: Vec<GenericParam>,
    pub fields: Vec<Field>,
    pub attributes: Vec<Attribute>,
    pub invariants: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Scroll {
    pub name: String,
    pub variants: Vec<Variant>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Option<Vec<Field>>, // for data-carrying variants
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Weave {
    pub name: String,
    pub associated_types: Vec<String>,
    pub methods: Vec<Forge>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Impl {
    pub target: Type,
    pub weave: Option<Type>,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub ty: Type,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Ward {
    pub name: String,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Draw {
    pub path: Vec<String>,
    pub is_wildcard: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String, Span),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Member(Box<Expr>, String, Span),
    Path(Vec<String>, Vec<Type>), // For turbofish/qualified paths
    Block(Block),
    Given {
        cond: Box<Expr>,
        then_block: Block,
        else_expr: Option<Box<Expr>>,
    },
    Branch {
        target: Box<Expr>,
        arms: Vec<Arm>,
    },
    Loop(Block),
    While {
        cond: Box<Expr>,
        body: Block,
    },
    Each {
        var: String,
        iter: Box<Expr>,
        body: Block,
    },
    Raw(Box<Expr>),
    Bind {
        params: Vec<String>,
        body: Box<Expr>,
    },
    StructLiteral {
        path: Type,
        fields: Vec<(String, Expr)>,
    },
    Return(Box<Expr>),
    Next,
    Break,
    Zone {
        name: String,
        body: Block,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i128),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Pipeline,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Deref,
    Ref(bool), // bool is mut
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>, // Trailing expression
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let {
        name: String,
        ty: Option<Type>,
        init: Option<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Type {
    Prim(String),
    Path(Vec<String>, Vec<Type>),
    Optional(Box<Type>),
    Cascade(Box<Type>),
    Pointer(Box<Type>, bool), // bool is mut
    Witness(Box<Type>),
    SelfType,
    Error,
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Arm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Ident(String),
    Variant(String, Vec<Pattern>),
    Literal(Literal),
    Struct {
        path: Type,
        fields: Vec<(String, Pattern)>,
    },
    Wildcard,
}
