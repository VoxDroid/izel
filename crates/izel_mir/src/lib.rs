//! Mid-level Intermediate Representation (MIR) for Izel.

use petgraph::graph::{DiGraph, NodeIndex};

pub type BlockId = NodeIndex;

pub mod lower;

pub struct MirBody {
    pub blocks: DiGraph<BasicBlock, ControlFlow>,
    pub entry: BlockId,
    pub locals: Vec<LocalData>,
}

pub struct LocalData {
    pub name: String,
    // Add type information later
}

pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Assign(Place, Rvalue),
    Call(Place, String, Vec<Operand>),
    StorageLive(Local),
    StorageDead(Local),
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Return,
    Goto(BlockId),
    SwitchInt(Operand, Vec<(u128, BlockId)>, BlockId),
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Local(pub usize);

#[derive(Debug, Clone)]
pub struct Place {
    pub local: Local,
    // Add projections later for fields/deref
}

#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnOp, Operand),
}

#[derive(Debug, Clone)]
pub enum Operand {
    Copy(Place),
    Move(Place),
    Constant(Constant),
}

#[derive(Debug, Clone)]
pub enum Constant {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Not, Neg,
}

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Unconditional,
    Conditional(bool),
}

impl MirBody {
    pub fn new() -> Self {
        let mut blocks = DiGraph::new();
        let entry = blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });
        Self { blocks, entry, locals: Vec::new() }
    }
}
