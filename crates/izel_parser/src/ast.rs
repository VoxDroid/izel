use izel_span::Span;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    Open,
    Hidden,
    Pkg,
    PkgPath(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Forge(Forge),
    Shape(Shape),
    Scroll(Scroll),
    Weave(Weave),
    Dual(Dual),
    Impl(Impl),
    Alias(Alias),
    Ward(Ward),
    Draw(Draw),
    Static(Static),
    Echo(Echo),
    Bridge(Bridge),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Forge {
    pub name: String,
    pub visibility: Visibility,
    pub is_flow: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub default_value: Option<Expr>,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dual {
    pub name: String,
    pub visibility: Visibility,
    pub generic_params: Vec<GenericParam>,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shape {
    pub name: String,
    pub visibility: Visibility,
    pub generic_params: Vec<GenericParam>,
    pub fields: Vec<Field>,
    pub attributes: Vec<Attribute>,
    pub invariants: Vec<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    pub name: String,
    pub visibility: Visibility,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scroll {
    pub name: String,
    pub visibility: Visibility,
    pub variants: Vec<Variant>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variant {
    pub name: String,
    pub fields: Option<Vec<Field>>, // for data-carrying variants
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Weave {
    pub name: String,
    pub visibility: Visibility,
    pub parents: Vec<Type>,
    pub associated_types: Vec<String>,
    pub methods: Vec<Forge>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Impl {
    pub target: Type,
    pub weave: Option<Type>,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alias {
    pub name: String,
    pub visibility: Visibility,
    pub ty: Type,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ward {
    pub name: String,
    pub visibility: Visibility,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Draw {
    pub path: Vec<String>,
    pub is_wildcard: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Static {
    pub name: String,
    pub visibility: Visibility,
    pub ty: Type,
    pub value: Option<Expr>,
    pub is_mut: bool,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Echo {
    pub body: Block,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bridge {
    pub abi: Option<String>,
    pub items: Vec<Item>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Literal(Literal),
    Ident(String, Span),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Call(Box<Expr>, Vec<Arg>),
    Member(Box<Expr>, String, Span),
    Path(Vec<String>, Vec<GenericArg>), // For turbofish/qualified paths
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
    Cascade {
        expr: Box<Expr>,
        context: Option<Box<Expr>>,
    },
    Seek {
        body: Block,
        catch_var: Option<String>,
        catch_body: Option<Block>,
    },
    Tide(Box<Expr>),
    WitnessNew(Box<GenericArg>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arg {
    pub label: Option<String>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i128),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::Int(a), Literal::Int(b)) => a == b,
            (Literal::Float(a), Literal::Float(b)) => a.to_bits() == b.to_bits(),
            (Literal::Str(a), Literal::Str(b)) => a == b,
            (Literal::Bool(a), Literal::Bool(b)) => a == b,
            (Literal::Nil, Literal::Nil) => true,
            _ => false,
        }
    }
}

impl Eq for Literal {}

impl Hash for Literal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Literal::Int(v) => v.hash(state),
            Literal::Float(v) => v.to_bits().hash(state),
            Literal::Str(v) => v.hash(state),
            Literal::Bool(v) => v.hash(state),
            Literal::Nil => 0.hash(state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
    Deref,
    Ref(bool), // bool is mut
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub expr: Option<Box<Expr>>, // Trailing expression
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stmt {
    Expr(Expr),
    Let {
        pat: Pattern,
        ty: Option<Type>,
        init: Option<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GenericArg {
    Type(Type),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Prim(String),
    Path(Vec<String>, Vec<GenericArg>),
    Optional(Box<Type>),
    Cascade(Box<Type>),
    Pointer(Box<Type>, bool), // bool is mut
    Witness(Box<GenericArg>),
    SelfType,
    Error,
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Ident(String, bool), // name, is_mut (e.g. ~x)
    Variant(String, Vec<Pattern>),
    Literal(Literal),
    Struct {
        path: Type,
        fields: Vec<(String, Pattern)>,
    },
    Tuple(Vec<Pattern>),
    Slice(Vec<Pattern>),
    Rest(String),     // e.g. ...tail
    Or(Vec<Pattern>), // e.g. A | B
    Wildcard,
}

pub trait AlphaEq {
    fn alpha_eq(&self, other: &Self) -> bool;
}

impl AlphaEq for Expr {
    fn alpha_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expr::Literal(a), Expr::Literal(b)) => a == b,
            (Expr::Ident(a, _), Expr::Ident(b, _)) => a == b,
            (Expr::Binary(op1, l1, r1), Expr::Binary(op2, l2, r2)) => {
                op1 == op2 && l1.alpha_eq(l2) && r1.alpha_eq(r2)
            }
            (Expr::Unary(op1, e1), Expr::Unary(op2, e2)) => op1 == op2 && e1.alpha_eq(e2),
            (Expr::Call(c1, a1), Expr::Call(c2, a2)) => {
                c1.alpha_eq(c2)
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Expr::Member(o1, f1, _), Expr::Member(o2, f2, _)) => o1.alpha_eq(o2) && f1 == f2,
            (Expr::Path(p1, g1), Expr::Path(p2, g2)) => {
                p1 == p2
                    && g1.len() == g2.len()
                    && g1.iter().zip(g2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Expr::Block(b1), Expr::Block(b2)) => b1.alpha_eq(b2),
            (
                Expr::Given {
                    cond: c1,
                    then_block: t1,
                    else_expr: e1,
                },
                Expr::Given {
                    cond: c2,
                    then_block: t2,
                    else_expr: e2,
                },
            ) => {
                c1.alpha_eq(c2)
                    && t1.alpha_eq(t2)
                    && match (e1, e2) {
                        (Some(x), Some(y)) => x.alpha_eq(y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (
                Expr::Branch {
                    target: t1,
                    arms: a1,
                },
                Expr::Branch {
                    target: t2,
                    arms: a2,
                },
            ) => {
                t1.alpha_eq(t2)
                    && a1.len() == a2.len()
                    && a1.iter().zip(a2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Expr::Loop(b1), Expr::Loop(b2)) => b1.alpha_eq(b2),
            (Expr::While { cond: c1, body: b1 }, Expr::While { cond: c2, body: b2 }) => {
                c1.alpha_eq(c2) && b1.alpha_eq(b2)
            }
            (
                Expr::Each {
                    var: v1,
                    iter: i1,
                    body: b1,
                },
                Expr::Each {
                    var: v2,
                    iter: i2,
                    body: b2,
                },
            ) => v1 == v2 && i1.alpha_eq(i2) && b1.alpha_eq(b2),
            (Expr::Raw(e1), Expr::Raw(e2)) => e1.alpha_eq(e2),
            (
                Expr::Bind {
                    params: p1,
                    body: b1,
                },
                Expr::Bind {
                    params: p2,
                    body: b2,
                },
            ) => p1 == p2 && b1.alpha_eq(b2),
            (
                Expr::StructLiteral {
                    path: p1,
                    fields: f1,
                },
                Expr::StructLiteral {
                    path: p2,
                    fields: f2,
                },
            ) => {
                p1.alpha_eq(p2)
                    && f1.len() == f2.len()
                    && f1
                        .iter()
                        .zip(f2.iter())
                        .all(|((n1, e1), (n2, e2))| n1 == n2 && e1.alpha_eq(e2))
            }
            (Expr::Return(e1), Expr::Return(e2)) => e1.alpha_eq(e2),
            (Expr::Next, Expr::Next) => true,
            (Expr::Break, Expr::Break) => true,
            (Expr::Zone { name: n1, body: b1 }, Expr::Zone { name: n2, body: b2 }) => {
                n1 == n2 && b1.alpha_eq(b2)
            }
            (
                Expr::Cascade {
                    expr: e1,
                    context: c1,
                },
                Expr::Cascade {
                    expr: e2,
                    context: c2,
                },
            ) => {
                e1.alpha_eq(e2)
                    && match (c1, c2) {
                        (Some(x), Some(y)) => x.alpha_eq(y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (
                Expr::Seek {
                    body: b1,
                    catch_var: v1,
                    catch_body: cb1,
                },
                Expr::Seek {
                    body: b2,
                    catch_var: v2,
                    catch_body: cb2,
                },
            ) => {
                b1.alpha_eq(b2)
                    && v1 == v2
                    && match (cb1, cb2) {
                        (Some(x), Some(y)) => x.alpha_eq(y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            (Expr::Tide(e1), Expr::Tide(e2)) => e1.alpha_eq(e2),
            (Expr::WitnessNew(a1), Expr::WitnessNew(a2)) => a1.alpha_eq(a2),
            _ => false,
        }
    }
}

impl AlphaEq for Arg {
    fn alpha_eq(&self, other: &Self) -> bool {
        self.label == other.label && self.value.alpha_eq(&other.value)
    }
}

impl AlphaEq for Block {
    fn alpha_eq(&self, other: &Self) -> bool {
        self.stmts.len() == other.stmts.len()
            && self
                .stmts
                .iter()
                .zip(other.stmts.iter())
                .all(|(a, b)| a.alpha_eq(b))
            && match (&self.expr, &other.expr) {
                (Some(e1), Some(e2)) => e1.alpha_eq(e2),
                (None, None) => true,
                _ => false,
            }
    }
}

impl AlphaEq for Stmt {
    fn alpha_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Stmt::Expr(e1), Stmt::Expr(e2)) => e1.alpha_eq(e2),
            (
                Stmt::Let {
                    pat: p1,
                    ty: t1,
                    init: i1,
                    ..
                },
                Stmt::Let {
                    pat: p2,
                    ty: t2,
                    init: i2,
                    ..
                },
            ) => {
                p1.alpha_eq(p2)
                    && match (t1, t2) {
                        (Some(x), Some(y)) => x.alpha_eq(y),
                        (None, None) => true,
                        _ => false,
                    }
                    && match (i1, i2) {
                        (Some(x), Some(y)) => x.alpha_eq(y),
                        (None, None) => true,
                        _ => false,
                    }
            }
            _ => false,
        }
    }
}

impl AlphaEq for Pattern {
    fn alpha_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Pattern::Ident(n1, m1), Pattern::Ident(n2, m2)) => n1 == n2 && m1 == m2,
            (Pattern::Variant(n1, p1), Pattern::Variant(n2, p2)) => {
                n1 == n2
                    && p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Pattern::Literal(l1), Pattern::Literal(l2)) => l1 == l2,
            (
                Pattern::Struct {
                    path: p1,
                    fields: f1,
                },
                Pattern::Struct {
                    path: p2,
                    fields: f2,
                },
            ) => {
                p1.alpha_eq(p2)
                    && f1.len() == f2.len()
                    && f1
                        .iter()
                        .zip(f2.iter())
                        .all(|((n1, p1), (n2, p2))| n1 == n2 && p1.alpha_eq(p2))
            }
            (Pattern::Tuple(p1), Pattern::Tuple(p2)) => {
                p1.len() == p2.len() && p1.iter().zip(p2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Pattern::Slice(p1), Pattern::Slice(p2)) => {
                p1.len() == p2.len() && p1.iter().zip(p2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Pattern::Rest(n1), Pattern::Rest(n2)) => n1 == n2,
            (Pattern::Or(p1), Pattern::Or(p2)) => {
                p1.len() == p2.len() && p1.iter().zip(p2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Pattern::Wildcard, Pattern::Wildcard) => true,
            _ => false,
        }
    }
}

impl AlphaEq for Type {
    fn alpha_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Type::Prim(p1), Type::Prim(p2)) => p1 == p2,
            (Type::Path(p1, g1), Type::Path(p2, g2)) => {
                p1 == p2
                    && g1.len() == g2.len()
                    && g1.iter().zip(g2.iter()).all(|(x, y)| x.alpha_eq(y))
            }
            (Type::Optional(t1), Type::Optional(t2)) => t1.alpha_eq(t2),
            (Type::Cascade(t1), Type::Cascade(t2)) => t1.alpha_eq(t2),
            (Type::Pointer(t1, m1), Type::Pointer(t2, m2)) => m1 == m2 && t1.alpha_eq(t2),
            (Type::Witness(a1), Type::Witness(a2)) => a1.alpha_eq(a2),
            (Type::SelfType, Type::SelfType) => true,
            (Type::Error, Type::Error) => true,
            (
                Type::Function {
                    params: p1,
                    ret: r1,
                    effects: e1,
                },
                Type::Function {
                    params: p2,
                    ret: r2,
                    effects: e2,
                },
            ) => {
                p1.len() == p2.len()
                    && p1.iter().zip(p2.iter()).all(|(x, y)| x.alpha_eq(y))
                    && r1.alpha_eq(r2)
                    && e1 == e2
            }
            _ => false,
        }
    }
}

impl AlphaEq for GenericArg {
    fn alpha_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (GenericArg::Type(t1), GenericArg::Type(t2)) => t1.alpha_eq(t2),
            (GenericArg::Expr(e1), GenericArg::Expr(e2)) => e1.alpha_eq(e2),
            _ => false,
        }
    }
}

impl AlphaEq for Arm {
    fn alpha_eq(&self, other: &Self) -> bool {
        self.pattern.alpha_eq(&other.pattern)
            && match (&self.guard, &other.guard) {
                (Some(g1), Some(g2)) => g1.alpha_eq(g2),
                (None, None) => true,
                _ => false,
            }
            && self.body.alpha_eq(&other.body)
    }
}
