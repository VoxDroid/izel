//! Type checking and inference for Izel.

use izel_resolve::DefId;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Function { params: Vec<Type>, ret: Box<Type> },
    Error,
}

pub struct TypeChecker {
    pub types: FxHashMap<DefId, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            types: FxHashMap::default(),
        }
    }

    pub fn check_source_file(&mut self) {
        // Mock check
    }
}
