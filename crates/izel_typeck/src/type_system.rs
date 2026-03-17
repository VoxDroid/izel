use crate::DefId;

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
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    F32, F64,
    Bool,
    Str,
    Void,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectSet {
    /// A concrete set of effects
    Concrete(Vec<Effect>),
    /// An effect variable (for row polymorphism)
    Var(usize),
    /// A row of effects + a tail (row poly)
    Row(Vec<Effect>, Box<EffectSet>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    IO,
    Alloc,
    Mut,
    Pure,
    User(String),
}

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
}
