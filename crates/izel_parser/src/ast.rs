use izel_span::Span;

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String, Span),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    // Desugared forms
    Given(Box<Expr>, Box<Block>, Option<Box<Expr>>),
    Branch(Box<Expr>, Vec<Arm>),
    // ...
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
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Gt, Le, Ge,
    // ...
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Let(String, Option<Type>, Option<Expr>, Span),
}

#[derive(Debug, Clone)]
pub enum Type {
    Path(String),
    Optional(Box<Type>),
    Pointer(Box<Type>, bool), // bool is mut (tilde)
}

#[derive(Debug, Clone)]
pub struct Arm {
    pub pattern: Expr, // simplistic
    pub body: Expr,
}
