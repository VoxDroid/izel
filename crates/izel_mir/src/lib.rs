//! Mid-level Intermediate Representation (MIR) for Izel.
// Forced rebuild 1

use petgraph::graph::{DiGraph, NodeIndex};

pub type BlockId = NodeIndex;

pub mod lower;
pub mod optim;

use izel_typeck::type_system::Type;

pub struct MirBody {
    pub blocks: DiGraph<BasicBlock, ControlFlow>,
    pub entry: BlockId,
    pub locals: Vec<LocalData>,
}

pub struct LocalData {
    pub name: String,
    pub ty: Type,
}

pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Assign(Local, Rvalue),
    Phi(Local, Vec<(BlockId, Local)>),
    Call(Local, String, Vec<Operand>),
    StorageLive(Local),
    StorageDead(Local),
    /// Runtime contract assertion: if operand is false, abort with message.
    Assert(Operand, String),
    /// Enter a memory zone — allocator becomes active.
    ZoneEnter(String),
    /// Exit a memory zone — bulk deallocation.
    ZoneExit(String),
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Option<Operand>),
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
    Ref(Local, bool), // bool is mut
}

#[derive(Debug, Clone)]
pub enum Operand {
    Copy(Local),
    Move(Local),
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
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Unconditional,
    Conditional(bool),
}

impl Default for MirBody {
    fn default() -> Self {
        Self::new()
    }
}

impl MirBody {
    pub fn new() -> Self {
        let mut blocks = DiGraph::new();
        let entry = blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });
        Self {
            blocks,
            entry,
            locals: Vec::new(),
        }
    }
}
