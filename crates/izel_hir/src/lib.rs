use izel_parser::ast;
use izel_resolve::DefId;
use izel_span::Span;
use izel_typeck::type_system::Type;

pub mod lower;

#[derive(Debug, Clone)]
pub struct HirModule {
    pub items: Vec<HirItem>,
}

#[derive(Debug, Clone)]
pub enum HirItem {
    Forge(Box<HirForge>),
    Shape(HirShape),
    Scroll(HirScroll),
}

#[derive(Debug, Clone)]
pub struct HirShape {
    pub name: String,
    pub def_id: DefId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirScroll {
    pub name: String,
    pub def_id: DefId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirForge {
    pub name: String,
    pub def_id: DefId,
    pub params: Vec<HirParam>,
    pub ret_type: Type,
    pub body: Option<HirBlock>,
    pub requires: Vec<HirExpr>,
    pub ensures: Vec<HirExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub name: String,
    pub def_id: DefId,
    pub ty: Type,
    pub default_value: Option<HirExpr>,
    pub is_variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirBlock {
    pub stmts: Vec<HirStmt>,
    pub expr: Option<Box<HirExpr>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum HirStmt {
    Let {
        name: String,
        def_id: DefId,
        ty: Type,
        init: Option<HirExpr>,
        span: Span,
    },
    Assign {
        def_id: DefId,
        expr: HirExpr,
        span: Span,
    },
    Expr(HirExpr),
}

#[derive(Debug, Clone)]
pub enum HirExpr {
    Literal(ast::Literal),
    Ident(DefId, Type, Span),
    Binary(ast::BinaryOp, Box<HirExpr>, Box<HirExpr>, Type),
    Unary(ast::UnaryOp, Box<HirExpr>, Type),
    Call(Box<HirExpr>, Vec<HirExpr>, Vec<HirExpr>, Type),
    Given {
        cond: Box<HirExpr>,
        then_block: HirBlock,
        else_expr: Option<Box<HirExpr>>,
        ty: Type,
    },
    While {
        cond: Box<HirExpr>,
        body: HirBlock,
    },
    Return(Option<Box<HirExpr>>),
    Zone {
        name: String,
        body: HirBlock,
        ty: Type,
    },
}
