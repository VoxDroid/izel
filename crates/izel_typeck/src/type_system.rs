use crate::DefId;
use izel_parser::ast;

/// Built-in witness type kinds.
/// Each represents a zero-cost compile-time proof that a predicate holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinWitness {
    /// `NonZero<T>` — value is proven non-zero
    NonZero,
    /// `InBounds<T>` — index is proven valid for a collection
    InBounds,
    /// `Sorted<T>` — collection is proven to be sorted
    Sorted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// Primitive types
    Prim(PrimType),

    /// User defined shapes/scrolls/duals
    Adt(DefId),

    /// Optional types (?T)
    Optional(Box<Type>),

    /// Cascade types (T!)
    Cascade(Box<Type>),

    /// Pointer types (*T or *~T)
    Pointer(Box<Type>, bool, Lifetime), // bool is mut

    /// Functions
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectSet,
    },

    /// Type variables (for inference)
    Var(usize),

    /// Generic parameters (<T>)
    Param(String),

    /// Tuple or anonymous shapes
    Static(Vec<(String, Type)>),

    /// Associated type (e.g. Iterator::Item)
    Assoc(Box<Type>, String),

    /// Witness types (Witness<P>)
    Witness(Box<Type>),

    /// Built-in witness types (NonZero<T>, InBounds<T>, Sorted<T>)
    BuiltinWitness(BuiltinWitness, Box<Type>),

    /// Type-level predicate (e.g. n > 0)
    Predicate(ast::Expr),

    /// Error sentinel
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Lifetime {
    /// Static lifetime ('static)
    Static,
    /// Named lifetime parameter ('a)
    Param(String),
    /// Anonymous/elided lifetime
    Anonymous(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Bool,
    Str,
    Void,
    None,
    ZoneAllocator,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectSet {
    /// A concrete set of effects
    Concrete(Vec<Effect>),
    /// An effect variable (for row polymorphism)
    Var(usize),
    /// A row of effects + a tail (row poly)
    Row(Vec<Effect>, Box<EffectSet>),
    /// A named effect parameter (for generic effects)
    Param(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    IO,
    Net,
    Alloc,
    Panic,
    Unsafe,
    Time,
    Rand,
    Env,
    Ffi,
    Thread,
    Mut,
    Pure,
    User(String),
}

use izel_parser::ast::Expr;

#[derive(Debug, Clone)]
pub struct Scheme {
    /// Anonymous inference variables to generalize
    pub vars: Vec<usize>,
    /// Anonymous effect variables to generalize
    pub effect_vars: Vec<usize>,
    /// Named generic parameters (<T>)
    pub names: Vec<String>,
    /// Bounds for generic parameters: (param_name, weave_name)
    pub bounds: Vec<(String, String)>,
    pub ty: Type,
    pub param_names: Vec<String>,
    pub requires: Vec<Expr>,
    pub ensures: Vec<Expr>,
    pub intrinsic: Option<String>,
    pub visibility: ast::Visibility,
}
