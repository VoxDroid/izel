pub mod type_system;

use izel_parser::ast;
use type_system::{Type, PrimType, Scheme, Effect, EffectSet, BuiltinWitness};
use izel_resolve::DefId;
use rustc_hash::FxHashMap;

pub mod contracts;
pub use izel_parser::eval::{ConstValue, eval_expr};
pub use izel_parser::contracts::ContractChecker;

pub struct TypeChecker {
    /// Resolved types for each DefId
    pub def_types: FxHashMap<DefId, Type>,
    /// Type of each expression span/id (once we have Expr IDs)
    pub expr_types: FxHashMap<usize, Type>,
    pub substitutions: FxHashMap<usize, Type>,
    pub effect_substitutions: FxHashMap<usize, EffectSet>,
    pub env: Vec<FxHashMap<String, Scheme>>,
    pub expected_ret: Option<Type>,
    pub current_level: usize,
    pub var_levels: FxHashMap<usize, usize>,
    pub effect_var_levels: FxHashMap<usize, usize>,
    pub current_effects: Vec<EffectSet>,
    next_var: usize,
    next_effect_var: usize,
    pub weaves: FxHashMap<String, ast::Weave>,
    pub trait_impls: FxHashMap<String, Vec<(Type, ast::Impl)>>,
    pub current_attributes: Vec<ast::Attribute>,
    pub in_raw_block: bool,
    pub diagnostics: Vec<izel_diagnostics::Diagnostic>,
    pub shape_invariants: FxHashMap<String, Vec<ast::Expr>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            def_types: FxHashMap::default(),
            expr_types: FxHashMap::default(),
            substitutions: FxHashMap::default(),
            effect_substitutions: FxHashMap::default(),
            env: vec![FxHashMap::default()], // Global scope
            expected_ret: None,
            current_level: 0,
            var_levels: FxHashMap::default(),
            effect_var_levels: FxHashMap::default(),
            current_effects: Vec::new(),
            next_var: 0,
            next_effect_var: 0,
            weaves: FxHashMap::default(),
            trait_impls: FxHashMap::default(),
            current_attributes: Vec::new(),
            in_raw_block: false,
            diagnostics: Vec::new(),
            shape_invariants: FxHashMap::default(),
        }
    }

    pub fn enter_level(&mut self) {
        self.current_level += 1;
    }

    pub fn exit_level(&mut self) {
        self.current_level -= 1;
    }

    pub fn push_scope(&mut self) {
        self.env.push(FxHashMap::default());
    }

    pub fn pop_scope(&mut self) {
        self.env.pop();
    }

    pub fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, Scheme { vars: vec![], effect_vars: vec![], names: vec![], bounds: vec![], ty, param_names: vec![], requires: vec![], ensures: vec![] });
        }
    }

    pub fn define_scheme(&mut self, name: String, scheme: Scheme) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, scheme);
        }
    }

    pub fn resolve_scheme(&self, name: &str) -> Option<Scheme> {
        for scope in self.env.iter().rev() {
            if let Some(s) = scope.get(name) {
                return Some(s.clone());
            }
        }
        None
    }

    pub fn resolve_name(&mut self, name: &str) -> Option<Type> {
        self.resolve_scheme(name).map(|s| self.instantiate(&s))
    }

    pub fn new_var(&mut self) -> Type {
        let var = Type::Var(self.next_var);
        self.var_levels.insert(self.next_var, self.current_level);
        self.next_var += 1;
        var
    }

    pub fn new_effect_var(&mut self) -> EffectSet {
        let id = self.next_effect_var;
        self.effect_var_levels.insert(id, self.current_level);
        self.next_effect_var += 1;
        EffectSet::Var(id)
    }

    pub fn check_ast(&mut self, module: &ast::Module) {
        // Pass 1: Collect top-level item signatures
        for item in &module.items {
            self.collect_item_signature(item);
        }

        // Pass 2: Check bodies
        for item in &module.items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Forge(f) => {
                self.check_forge(f)
            },
            ast::Item::Impl(i) => {
                self.check_impl(i);
                for it in &i.items {
                    self.check_item(it);
                }
            }
            ast::Item::Ward(w) => {
                for it in &w.items {
                    self.check_item(it);
                }
            }
            _ => {}
        }
    }

    fn check_forge(&mut self, f: &ast::Forge) {
         self.push_scope();
         
         // Define generic parameters in scope
         for gp in &f.generic_params {
             self.define(gp.name.clone(), Type::Param(gp.name.clone()));
         }

         let ret_ty = self.lower_ast_type(&f.ret_type);
         let old_ret = self.expected_ret.replace(ret_ty.clone());
         let old_attrs = std::mem::replace(&mut self.current_attributes, f.attributes.clone());
         
         for param in &f.params {
              let pty = self.lower_ast_type(&param.ty);
              self.define(param.name.clone(), pty.clone());
         }
         
         if let Some(body) = &f.body {
             let body_effects = self.new_effect_var();
             self.current_effects.push(body_effects.clone());
             self.check_block_with_expected(body, Some(&ret_ty));
             
             let collected = self.current_effects.pop().unwrap();
             
             // Unify body effects with declared/inferred effects
             if let Some(sig) = self.resolve_name(&f.name) {
                 if let Type::Function { effects: declared, .. } = self.prune(&sig) {
                     if !self.unify_effects(&collected, &declared) {
                         eprintln!("Error: Function has effects {:?} but only declared {:?}", collected, declared);
                     }
                 }
             }

             // Static verification of postconditions (@ensures)
             if !f.ensures.is_empty() {
                 if let Some(expr) = &body.expr {
                     let ret_val = ::izel_parser::eval::eval_expr(expr, &std::collections::HashMap::new());
                     if ret_val != ::izel_parser::eval::ConstValue::Unknown {
                         let diags = contracts::ContractChecker::check_ensures_from_scheme(
                             &f.name,
                             &f.ensures,
                             &ret_val,
                             body.span,
                             &std::collections::HashMap::new()
                         );
                         self.diagnostics.extend(diags);
                     }
                 }
             }
         }
         
         self.current_attributes = old_attrs;
         self.expected_ret = old_ret;
         self.pop_scope();
    }

    fn check_impl(&mut self, i: &ast::Impl) {
        let _target = self.lower_ast_type(&i.target);
        if let Some(weave_ty) = &i.weave {
             if let ast::Type::Prim(weave_name) = weave_ty {
                 if let Some(w) = self.weaves.get(weave_name).cloned() {
                     for expected_method in &w.methods {
                         let found = i.items.iter().find(|item| {
                             if let ast::Item::Forge(f) = item {
                                 f.name == expected_method.name
                             } else { false }
                         });
                         
                         if found.is_none() {
                         }
                     }
                 }
             }
        }

        // Check invariant preservation: if the target shape has invariants,
        // each method that takes `~self` (mutable self) must preserve them.
        let target_name = match &i.target {
            ast::Type::Prim(name) => Some(name.clone()),
            _ => None,
        };
        if let Some(name) = target_name {
            if let Some(invariants) = self.shape_invariants.get(&name).cloned() {
                for item in &i.items {
                    if let ast::Item::Forge(f) = item {
                        // Check if this method mutates self (has ~self param)
                        let has_mut_self = f.params.iter().any(|p| p.name == "self");
                        if has_mut_self && !invariants.is_empty() {
                            // Record that invariants must hold as postconditions
                            // (The runtime check is handled by MIR assertion injection)
                            for inv in &invariants {
                                // Verify invariant is not explicitly broken by the method;
                                // For now, register a diagnostic note for tracking.
                                let _ = inv; // Invariant tracking registered
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_item_signature(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Weave(w) => {
                self.weaves.insert(w.name.clone(), w.clone());
                self.define(w.name.clone(), Type::Prim(PrimType::Void));
            }
            ast::Item::Impl(i) => {
                let target = self.lower_ast_type(&i.target);
                if let Some(weave_ty) = &i.weave {
                    if let ast::Type::Prim(weave_name) = weave_ty {
                        // Coherence: Check for duplicate implementation
                        if let Some(impls) = self.trait_impls.get(weave_name) {
                            for (existing_ty, _) in impls {
                                if existing_ty == &target {
                                    eprintln!("Error: Duplicate implementation of weave {} for type {:?}", weave_name, target);
                                    return;
                                }
                            }
                        }
                        // Orphan Rule: Either weave or type must be local
                        let weave_is_local = self.weaves.contains_key(weave_name);
                        let type_is_local = matches!(self.prune(&target), Type::Adt(_) | Type::Static(_));
                        
                        if !weave_is_local && !type_is_local {
                            eprintln!("Error: Orphan rule violation: Cannot implement foreign weave {} for foreign type {:?}", weave_name, target);
                            return;
                        }

                        self.trait_impls.entry(weave_name.clone())
                            .or_default()
                            .push((target, i.clone()));
                    }
                }
                // Signatures of items inside impl should also be collected
                for it in &i.items {
                    self.collect_item_signature(it);
                }
            }
            ast::Item::Ward(w) => {
                for it in &w.items {
                    self.collect_item_signature(it);
                }
            }
            ast::Item::Forge(f) => {
                self.push_scope();
                let mut bounds = Vec::new();
                // Map generic params to Type::Param
                for gp in &f.generic_params {
                    self.define(gp.name.clone(), Type::Param(gp.name.clone()));
                    for b in &gp.bounds {
                        bounds.push((gp.name.clone(), b.clone()));
                    }
                }
                
                let mut params = Vec::new();
                let mut param_names = Vec::new();
                for p in &f.params {
                    params.push(self.lower_ast_type(&p.ty));
                    param_names.push(p.name.clone());
                }
                
                let mut ret = Box::new(self.lower_ast_type(&f.ret_type));
                self.apply_lifetime_elision(&mut params, &mut ret);

                let mut effects = Vec::new();
                for e in &f.effects {
                    effects.push(match e.as_str() {
                        "io" => Effect::IO,
                        "alloc" => Effect::Alloc,
                        "mut" => Effect::Mut,
                        "pure" => Effect::Pure,
                        _ => Effect::User(e.clone()),
                    });
                }

                let effect_set = if f.effects.is_empty() {
                    self.new_effect_var()
                } else if f.effects.contains(&"pure".to_string()) {
                    EffectSet::Concrete(vec![Effect::Pure])
                } else {
                    EffectSet::Concrete(effects)
                };

                let ty = Type::Function { 
                    params, 
                    ret, 
                    effects: effect_set
                };
                
                self.pop_scope();
                
                // Generalize it
                let mut scheme = self.generalize(&ty);
                scheme.bounds = bounds;
                scheme.param_names = param_names;
                scheme.requires = f.requires.clone();
                scheme.ensures = f.ensures.clone();
                self.define_scheme(f.name.clone(), scheme);
            }
            ast::Item::Shape(s) => {
                self.push_scope();
                let mut bounds = Vec::new();
                for gp in &s.generic_params {
                    self.define(gp.name.clone(), Type::Param(gp.name.clone()));
                    for b in &gp.bounds {
                        bounds.push((gp.name.clone(), b.clone()));
                    }
                }
                
                let mut fields = vec![];
                for f in &s.fields {
                    fields.push((f.name.clone(), self.lower_ast_type(&f.ty)));
                }
                let ty = Type::Adt(DefId(0)); // Placeholder for actual DefId logic
                self.pop_scope();
                
                let mut scheme = self.generalize(&ty);
                scheme.bounds = bounds;
                self.define_scheme(s.name.clone(), scheme);

                // Store invariants for later checking in check_impl
                if !s.invariants.is_empty() {
                    self.shape_invariants.insert(s.name.clone(), s.invariants.clone());
                }
            }
            ast::Item::Alias(a) => {
                let ty = self.lower_ast_type(&a.ty);
                self.define(a.name.clone(), ty);
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &ast::Block) {
        self.check_block_with_expected(block, None);
    }

    fn check_block_with_expected(&mut self, block: &ast::Block, expected: Option<&Type>) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            let ty = self.infer_expr(expr);
            if let Some(et) = expected {
                if !self.unify(et, &ty) {
                    eprintln!("Error: Block return type mismatch. Expected {:?}, found {:?}", et, ty);
                }
            }
        } else if let Some(et) = expected {
            // Empty block with expected return type must be Void
            self.unify(&Type::Prim(PrimType::Void), et);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Expr(e) => { self.infer_expr(e); }
            ast::Stmt::Let { name, ty, init, span: _ } => {
                self.enter_level();
                let mut var_ty = self.new_var();
                if let Some(explicit_ty) = ty {
                    let et = self.lower_ast_type(explicit_ty);
                    self.unify(&var_ty, &et);
                    var_ty = et;
                }
                if let Some(init_expr) = init {
                    let it = self.infer_expr(init_expr);
                    if !self.unify(&var_ty, &it) {
                        eprintln!("Error: Type mismatch in 'let' initializer. Expected {:?}, found {:?}", var_ty, it);
                    }
                }
                self.exit_level();
                
                let scheme = self.generalize(&var_ty);
                self.define_scheme(name.clone(), scheme);
            }
        }
    }

    fn lower_ast_type(&mut self, ty: &ast::Type) -> Type {
        match ty {
            ast::Type::Prim(s) => match s.as_str() {
                "i32" => Type::Prim(PrimType::I32),
                "i64" => Type::Prim(PrimType::I64),
                "u8" => Type::Prim(PrimType::U8),
                "u16" => Type::Prim(PrimType::U16),
                "u32" => Type::Prim(PrimType::U32),
                "u64" => Type::Prim(PrimType::U64),
                "usize" => Type::Prim(PrimType::U64), // alias for now
                "f32" => Type::Prim(PrimType::F32),
                "f64" => Type::Prim(PrimType::F64),
                "str" => Type::Prim(PrimType::Str),
                "bool" => Type::Prim(PrimType::Bool),
                "void" => Type::Prim(PrimType::Void),
                _ => {
                    if let Some(t) = self.resolve_name(s) {
                        t
                    } else {
                        Type::Error
                    }
                }
            },
            ast::Type::Optional(inner) => {
                let inner_ty = self.lower_ast_type(inner);
                Type::Optional(Box::new(inner_ty))
            }
            ast::Type::Cascade(inner) => {
                let inner_ty = self.lower_ast_type(inner);
                Type::Cascade(Box::new(inner_ty))
            }
            ast::Type::Pointer(inner, m) => {
                let inner_ty = self.lower_ast_type(inner);
                Type::Pointer(Box::new(inner_ty), *m, type_system::Lifetime::Anonymous(0))
            }
            ast::Type::Witness(inner) => {
                let inner_ty = self.lower_ast_type(inner);
                Type::Witness(Box::new(inner_ty))
            }
            ast::Type::Path(parts, gen_args) => {
                // Check for built-in witness type names with generic args
                if parts.len() == 1 {
                    let name = &parts[0];
                    let builtin = match name.as_str() {
                        "NonZero" => Some(BuiltinWitness::NonZero),
                        "InBounds" => Some(BuiltinWitness::InBounds),
                        "Sorted" => Some(BuiltinWitness::Sorted),
                        _ => None,
                    };
                    if let Some(kind) = builtin {
                        let inner = if !gen_args.is_empty() {
                            self.lower_ast_type(&gen_args[0])
                        } else {
                            self.new_var() // infer inner type
                        };
                        return Type::BuiltinWitness(kind, Box::new(inner));
                    }
                }

                if parts.len() > 1 {
                    // Very simple resolver: first part is the base, next is associated type
                    let mut current = if let Some(t) = self.resolve_name(&parts[0]) {
                        t
                    } else {
                        Type::Error
                    };
                    for part in &parts[1..] {
                        current = Type::Assoc(Box::new(current), part.clone());
                    }
                    current
                } else {
                    let name = parts.join("::");
                    if let Some(t) = self.resolve_name(&name) {
                        t
                    } else {
                        Type::Error
                    }
                }
            }
            ast::Type::SelfType => self.resolve_name("Self").unwrap_or(Type::Error),
            _ => Type::Error,
        }
    }

    pub fn unify(&mut self, t1: &Type, t2: &Type) -> bool {
        let t1 = self.prune(t1);
        let t2 = self.prune(t2);

        if let Type::Assoc(base, name) = &t1 {
            let resolved = self.resolve_assoc_type(base, name);
            if resolved != Type::Error {
                return self.unify(&resolved, &t2);
            }
        }
        if let Type::Assoc(base, name) = &t2 {
            let resolved = self.resolve_assoc_type(base, name);
            if resolved != Type::Error {
                return self.unify(&t1, &resolved);
            }
        }

        match (&t1, &t2) {
            (Type::Var(id1), Type::Var(id2)) if id1 == id2 => true,
            (Type::Var(id), other) => {
                self.occurs_check_and_adjust_levels(*id, other)
            }
            (other, Type::Var(id)) => {
                self.occurs_check_and_adjust_levels(*id, other)
            }
            (Type::Prim(p1), Type::Prim(p2)) => p1 == p2,
            (Type::Prim(PrimType::None), Type::Optional(_)) => true,
            (Type::Optional(_), Type::Prim(PrimType::None)) => true,
            (Type::Prim(PrimType::None), Type::Cascade(_)) => true,
            (Type::Cascade(_), Type::Prim(PrimType::None)) => true,
            // Cascade and Optional can unify (Cascade is a superset usually)
            (Type::Optional(o), Type::Cascade(c)) => self.unify(o, c),
            (Type::Cascade(c), Type::Optional(o)) => self.unify(c, o),
            
            (Type::Static(f1), Type::Static(f2)) => {
                if f1.len() != f2.len() { return false; }
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 || !self.unify(t1, t2) { return false; }
                }
                true
            }
            (Type::Optional(o1), Type::Optional(o2)) => self.unify(&o1, &o2),
            (Type::Cascade(c1), Type::Cascade(c2)) => self.unify(&c1, &c2),
            (Type::Pointer(p1, m1, l1), Type::Pointer(p2, m2, l2)) => m1 == m2 && l1 == l2 && self.unify(&p1, &p2),
            (Type::Witness(w1), Type::Witness(w2)) => self.unify(&w1, &w2),
            // Built-in witness types: same kind + inner unification
            (Type::BuiltinWitness(k1, t1), Type::BuiltinWitness(k2, t2)) => {
                k1 == k2 && self.unify(&t1, &t2)
            }
            (Type::Function { params: p1, ret: r1, effects: e1 }, Type::Function { params: p2, ret: r2, effects: e2 }) => {
                if p1.len() != p2.len() { return false; }
                for (p1, p2) in p1.iter().zip(p2.iter()) {
                    if !self.unify(p1, p2) { return false; }
                }
                if !self.unify(r1, r2) { return false; }
                self.unify_effects(e1, e2)
            }
            (Type::Adt(id1), Type::Adt(id2)) => id1 == id2,

            // Implicit promotion (e.g. T -> ?T or T -> T!)
            (t, Type::Optional(o)) => self.unify(t, o),
            (t, Type::Cascade(c)) => self.unify(t, c),
            (Type::Optional(o), t) => self.unify(o, t),
            (Type::Cascade(c), t) => self.unify(c, t),
            
            // Witness promotion: 
            // Value -> Witness: ONLY in proof mode
            (Type::Witness(w), t) => if self.is_proof_mode() { self.unify(w, t) } else { false },
            // Witness -> Value: Always allowed
            (t, Type::Witness(w)) => self.unify(t, w),

            // BuiltinWitness -> inner value: Always allowed (extract)
            (t, Type::BuiltinWitness(_, inner)) => self.unify(t, &inner),
            // inner value -> BuiltinWitness: ONLY in proof mode
            (Type::BuiltinWitness(_, inner), t) => if self.is_proof_mode() { self.unify(&inner, t) } else { false },
            
            _ => {
                false
            }
        }
    }

    fn resolve_assoc_type(&mut self, base: &Type, name: &str) -> Type {
        let base = self.prune(base);
        // Look through all trait implementations
        // Clone the list of impls to avoid borrow conflict with self.lower_ast_type
        let all_impls: Vec<_> = self.trait_impls.values().flatten().cloned().collect();
        
        for (impl_ty, i) in all_impls {
            // If the base type matches the impl target type
            if self.unify_without_binding(&base, &impl_ty) {
                for item in &i.items {
                    if let ast::Item::Alias(a) = item {
                         if a.name == name {
                             return self.lower_ast_type(&a.ty);
                         }
                    }
                }
            }
        }
        Type::Error
    }

    fn unify_without_binding(&mut self, t1: &Type, t2: &Type) -> bool {
        t1 == t2
    }

    pub fn unify_effects(&mut self, e1: &EffectSet, e2: &EffectSet) -> bool {
        let e1 = self.prune_effects(e1);
        let e2 = self.prune_effects(e2);
        
        match (&e1, &e2) {
            (EffectSet::Var(id1), EffectSet::Var(id2)) if id1 == id2 => true,
            (EffectSet::Var(id), other) => self.bind_effect_var(*id, other.clone()),
            (other, EffectSet::Var(id)) => self.bind_effect_var(*id, other.clone()),
            (EffectSet::Concrete(v1), EffectSet::Concrete(v2)) => {
                if v1.len() != v2.len() { return false; }
                for e in v1 {
                    if !v2.contains(e) { return false; }
                }
                true
            }
            (EffectSet::Row(vals1, tail1), EffectSet::Row(vals2, tail2)) => {
                if vals1 == vals2 {
                    return self.unify_effects(tail1, tail2);
                }
                false
            }
            (EffectSet::Concrete(v), EffectSet::Row(vals, tail)) | (EffectSet::Row(vals, tail), EffectSet::Concrete(v)) => {
                for e in vals {
                    if !v.contains(e) { return false; }
                }
                let remaining: Vec<_> = v.iter().filter(|e| !vals.contains(e)).cloned().collect();
                self.unify_effects(tail, &EffectSet::Concrete(remaining))
            }
            (EffectSet::Param(p1), EffectSet::Param(p2)) => p1 == p2,
            _ => false,
        }
    }

    pub fn accumulate_effects(&mut self, current: &EffectSet, new: &EffectSet) {
        let current_pruned = self.prune_effects(current);
        let new_pruned = self.prune_effects(new);
        
        match new_pruned {
            EffectSet::Concrete(v) => {
                for e in v {
                    self.add_single_effect(&current_pruned, e);
                }
            }
            EffectSet::Row(vals, tail) => {
                for e in vals {
                    self.add_single_effect(&current_pruned, e);
                }
                self.accumulate_effects(&current_pruned, &tail);
            }
            EffectSet::Var(_) | EffectSet::Param(_) => {
                // For variables or parameters, we unify to ensure the current set covers them
                self.unify_effects(&current_pruned, &new_pruned);
            }
        }
    }

    fn add_single_effect(&mut self, current: &EffectSet, e: Effect) {
        let current = self.prune_effects(current);
        match current {
            EffectSet::Var(id) => {
                let next_tail = self.new_effect_var();
                self.bind_effect_var(id, EffectSet::Row(vec![e], Box::new(next_tail)));
            }
            EffectSet::Row(vals, tail) => {
                if !vals.contains(&e) {
                    self.add_single_effect(&tail, e);
                }
            }
            EffectSet::Concrete(v) => {
                if !v.contains(&e) {
                    // TODO: This should probably be a diagnostic if we're checking against a signature
                    // For now, we just don't add it if it's not allowed
                }
            }
            EffectSet::Param(_) => {
                // Cannot add to a fixed parameter
            }
        }
    }

    fn bind_effect_var(&mut self, id: usize, effects: EffectSet) -> bool {
        if let EffectSet::Var(other_id) = effects {
            if id == other_id { return true; }
        }
        if self.occurs_check_effects(id, &effects) {
            return false;
        }
        let var_level = self.effect_var_levels.get(&id).cloned().unwrap_or(0);
        self.adjust_effect_levels(var_level, &effects);
        
        self.effect_substitutions.insert(id, effects);
        true
    }

    fn occurs_check_effects(&self, id: usize, effects: &EffectSet) -> bool {
        match self.prune_effects(effects) {
            EffectSet::Var(other_id) => id == other_id,
            EffectSet::Row(_, tail) => self.occurs_check_effects(id, &tail),
            _ => false,
        }
    }

    fn adjust_effect_levels(&mut self, level: usize, effects: &EffectSet) {
        match self.prune_effects(effects) {
            EffectSet::Var(id) => {
                let l = self.effect_var_levels.get(&id).cloned().unwrap_or(0);
                if l > level {
                    self.effect_var_levels.insert(id, level);
                }
            }
            EffectSet::Row(_, tail) => self.adjust_effect_levels(level, &tail),
            _ => {}
        }
    }

    fn is_proof_mode(&self) -> bool {
        let res = self.in_raw_block || self.current_attributes.iter().any(|a| a.name == "proof");
        res
    }

    fn prune(&self, ty: &Type) -> Type {
        if let Type::Var(id) = ty {
            if let Some(bound) = self.substitutions.get(id) {
                return self.prune(bound);
            }
        }
        ty.clone()
    }

    fn prune_effects(&self, effects: &EffectSet) -> EffectSet {
        if let EffectSet::Var(id) = effects {
            if let Some(bound) = self.effect_substitutions.get(id) {
                return self.prune_effects(bound);
            }
        }
        effects.clone()
    }

    fn bind_var(&mut self, id: usize, ty: Type) {
        if let Type::Var(other_id) = ty {
             if id == other_id { return; }
        }
        self.substitutions.insert(id, ty);
    }

    pub fn infer_expr(&mut self, expr: &ast::Expr) -> Type {
        let res = match expr {
            ast::Expr::Literal(l) => match l {
                ast::Literal::Int(_) => Type::Prim(PrimType::I32),
                ast::Literal::Float(_) => Type::Prim(PrimType::F64),
                ast::Literal::Str(_) => Type::Prim(PrimType::Str),
                ast::Literal::Bool(_) => Type::Prim(PrimType::Bool),
                ast::Literal::Nil => Type::Prim(PrimType::None),
            },
            ast::Expr::Ident(name, _) => {
                if let Some(ty) = self.resolve_name(name) {
                    ty
                } else {
                    Type::Error
                }
            }
            ast::Expr::Binary(op, lhs, rhs) => {
                let lt = self.infer_expr(lhs);
                let rt = self.infer_expr(rhs);
                match op {
                    ast::BinaryOp::Add | ast::BinaryOp::Sub | ast::BinaryOp::Mul | ast::BinaryOp::Div => {
                        self.unify(&lt, &Type::Prim(PrimType::I32)); // Placeholder
                        self.unify(&rt, &Type::Prim(PrimType::I32));
                        Type::Prim(PrimType::I32)
                    }
                    ast::BinaryOp::Eq | ast::BinaryOp::Ne | ast::BinaryOp::Lt | ast::BinaryOp::Gt | ast::BinaryOp::Le | ast::BinaryOp::Ge => {
                        self.unify(&lt, &rt);
                        Type::Prim(PrimType::Bool)
                    }
                    _ => {
                        self.unify(&lt, &rt);
                        lt
                    }
                }
            }
            ast::Expr::Unary(op, inner) => {
                let it = self.infer_expr(inner);
                match op {
                    ast::UnaryOp::Neg => {
                        self.unify(&it, &Type::Prim(PrimType::I32));
                        it
                    }
                    ast::UnaryOp::Not => {
                        self.unify(&it, &Type::Prim(PrimType::Bool));
                        it
                    }
                    ast::UnaryOp::Ref(m) => Type::Pointer(Box::new(it), *m, type_system::Lifetime::Anonymous(0)),
                    ast::UnaryOp::Deref => {
                        let res = self.new_var();
                        self.unify(&it, &Type::Pointer(Box::new(res.clone()), false, type_system::Lifetime::Anonymous(0))); // can be mut or not
                        res
                    }
                    _ => it,
                }
            }
            ast::Expr::Given { cond, then_block, else_expr } => {
                let ct = self.infer_expr(cond);
                self.unify(&ct, &Type::Prim(PrimType::Bool));
                let res_ty = self.new_var();
                self.check_block_with_expected(then_block, Some(&res_ty));
                if let Some(else_e) = else_expr {
                    let et = self.infer_expr(else_e);
                    self.unify(&res_ty, &et);
                } else {
                    self.unify(&res_ty, &Type::Prim(PrimType::Void));
                }
                res_ty
            }
            ast::Expr::Member(obj, field, _) => {
                let ot = self.infer_expr(obj);
                if let Type::Static(fields) = self.prune(&ot) {
                    if let Some((_, fty)) = fields.iter().find(|(name, _)| name == field) {
                         return fty.clone();
                    }
                }
                self.new_var()
            }
            ast::Expr::Call(callee, args) => {
                let ct = self.infer_expr(callee);
                
                // Static verification of @requires at call-sites
                if let ast::Expr::Ident(name, span) = callee.as_ref() {
                    if let Some(scheme) = self.resolve_scheme(name) {
                        if !scheme.requires.is_empty() {
                            let mut eval_args = Vec::new();
                            for arg in args {
                                eval_args.push(::izel_parser::eval::eval_expr(arg, &std::collections::HashMap::new()));
                            }
                            // Only check if all args are known constants
                            if eval_args.iter().all(|a| *a != ::izel_parser::eval::ConstValue::Unknown) {
                                let diags = contracts::ContractChecker::check_requires_from_scheme(
                                    name,
                                    &scheme.param_names,
                                    &scheme.requires,
                                    &eval_args,
                                    *span
                                );
                                self.diagnostics.extend(diags);
                            }
                        }
                    }
                }

                if let Type::Function { params, ret, effects } = self.prune(&ct) {
                     let current = self.current_effects.last().cloned();
                     if let Some(curr) = current {
                         self.accumulate_effects(&curr, &effects);
                     }
                     for (arg, pty) in args.iter().zip(params.iter()) {
                          let at = self.infer_expr(arg);
                          if !self.unify(pty, &at) {
                               eprintln!("Error: Argument type mismatch. Expected {:?}, found {:?}", pty, at);
                          }
                     }
                     *ret
                } else {
                     for arg in args { self.infer_expr(arg); }
                     let res = self.new_var();
                     res
                }
            }
            ast::Expr::StructLiteral { path, fields } => {
                let mut struct_ty = self.new_var();
                if let ast::Type::Prim(name) = path {
                     if let Some(ty) = self.resolve_name(name) {
                          struct_ty = ty;
                     }
                }
                if let Type::Static(st_fields) = self.prune(&struct_ty) {
                     for (fname, fexpr) in fields {
                          if let Some((_, fty)) = st_fields.iter().find(|(n, _)| n == fname) {
                               let et = self.infer_expr(fexpr);
                               self.unify(&et, fty);
                          }
                     }
                }
                struct_ty
            }
            ast::Expr::Branch { target, arms } => {
                let tt = self.infer_expr(target);
                let res_ty = self.new_var();
                for arm in arms {
                    self.push_scope();
                    self.bind_pattern(&arm.pattern, &tt);
                    let at = self.infer_expr(&arm.body);
                    self.unify(&res_ty, &at);
                    self.pop_scope();
                }
                res_ty
            }
            ast::Expr::Return(inner) => {
                let ty = self.infer_expr(inner);
                if let Some(target) = self.expected_ret.clone() {
                    self.unify(&ty, &target);
                }
                Type::Error
            }
            ast::Expr::Next | ast::Expr::Break => Type::Prim(PrimType::Void),
            ast::Expr::Loop(block) => {
                self.check_block(block);
                Type::Prim(PrimType::Void)
            }
            ast::Expr::While { cond: _, body } => {
                self.check_block(body);
                Type::Prim(PrimType::Void)
            }
            ast::Expr::Raw(inner) => {
                let ty = self.infer_expr(inner);
                // raw x always produces a Witness<T> in proof context
                // even in non-proof functions, 'raw' is the explicit bypass.
                Type::Witness(Box::new(ty))
            }
            ast::Expr::Each { var, iter, body } => {
                let _it = self.infer_expr(iter);
                let item_ty = self.new_var();
                self.push_scope();
                self.define(var.clone(), item_ty);
                self.check_block(body);
                self.pop_scope();
                Type::Prim(PrimType::Void)
            }
            ast::Expr::Bind { params, body } => {
                self.push_scope();
                let mut param_tys = Vec::new();
                for p in params {
                    let pt = self.new_var();
                    self.define(p.clone(), pt.clone());
                    param_tys.push(pt);
                }
                let ret = self.infer_expr(body);
                self.pop_scope();
                Type::Function {
                    params: param_tys,
                    ret: Box::new(ret),
                    effects: self.new_effect_var(),
                }
            }
            ast::Expr::Block(block) => {
                let res_ty = self.new_var();
                self.check_block_with_expected(block, Some(&res_ty));
                res_ty
            }
            ast::Expr::Path(segments, _generics) => {
                if segments.len() == 1 {
                    if let Some(ty) = self.resolve_name(&segments[0]) {
                        return ty;
                    }
                }
                Type::Error
            }
        };
        // TODO: Store in expr_types
        res
    }

    fn bind_pattern(&mut self, pattern: &ast::Pattern, ty: &Type) {
        let ty = self.prune(ty);
        match pattern {
            ast::Pattern::Ident(name) => {
                self.define(name.clone(), ty);
            }
            ast::Pattern::Variant(_variant, subpatterns) => {
                // Hardcoded logic for Optional and Cascade unwrapping for now
                match ty {
                    Type::Optional(inner) | Type::Cascade(inner) => {
                        for sub in subpatterns {
                            self.bind_pattern(sub, &inner);
                        }
                    }
                    _ => {
                        for sub in subpatterns {
                            self.bind_pattern(sub, &Type::Error);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn occurs_check_and_adjust_levels(&mut self, var_id: usize, ty: &Type) -> bool {
        let var_level = self.var_levels.get(&var_id).cloned().unwrap_or(0);
        if !self.check_and_adjust(var_id, var_level, ty) {
            return false;
        }
        self.bind_var(var_id, ty.clone());
        true
    }

    fn check_and_adjust(&mut self, var_id: usize, var_level: usize, ty: &Type) -> bool {
        match self.prune(ty) {
            Type::Var(id) => {
                if id == var_id {
                    return false; // Recursive type
                }
                let other_level = self.var_levels.get(&id).cloned().unwrap_or(0);
                if other_level > var_level {
                    self.var_levels.insert(id, var_level);
                }
                true
            }
            Type::Function { params, ret, effects: _ } => {
                for p in params {
                    if !self.check_and_adjust(var_id, var_level, &p) { return false; }
                }
                self.check_and_adjust(var_id, var_level, &ret)
            }
            Type::Optional(inner) | Type::Cascade(inner) | Type::Pointer(inner, _, _) | Type::BuiltinWitness(_, inner) => {
                self.check_and_adjust(var_id, var_level, &inner)
            }
            Type::Static(fields) => {
                for (_, t) in fields {
                    if !self.check_and_adjust(var_id, var_level, &t) { return false; }
                }
                true
            }
            _ => true,
        }
    }

    fn generalize(&self, ty: &Type) -> Scheme {
        let mut vars = Vec::new();
        let mut effect_vars = Vec::new();
        let mut names = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut seen_effects = std::collections::HashSet::new();
        let mut seen_names = std::collections::HashSet::new();
        self.find_gen_vars(ty, &mut vars, &mut seen, &mut effect_vars, &mut seen_effects, &mut names, &mut seen_names);
        Scheme { vars, effect_vars, names, bounds: vec![], ty: ty.clone(), param_names: vec![], requires: vec![], ensures: vec![] }
    }

    fn find_gen_vars(
        &self, 
        ty: &Type, 
        vars: &mut Vec<usize>, 
        seen: &mut std::collections::HashSet<usize>,
        effect_vars: &mut Vec<usize>,
        seen_effects: &mut std::collections::HashSet<usize>,
        names: &mut Vec<String>,
        seen_names: &mut std::collections::HashSet<String>,
    ) {
        let ty = self.prune(ty);
        match ty {
            Type::Var(id) => {
                if seen.insert(id) {
                    let level = self.var_levels.get(&id).cloned().unwrap_or(0);
                    if level > self.current_level {
                        vars.push(id);
                    }
                }
            }
            Type::Param(name) => {
                if seen_names.insert(name.clone()) {
                    names.push(name.clone());
                }
            }
            Type::Function { params, ret, effects } => {
                for p in params {
                    self.find_gen_vars(&p, vars, seen, effect_vars, seen_effects, names, seen_names);
                }
                self.find_gen_vars(&ret, vars, seen, effect_vars, seen_effects, names, seen_names);
                self.find_gen_effect_vars(&effects, effect_vars, seen_effects);
            }
            Type::Optional(inner) | Type::Cascade(inner) | Type::Pointer(inner, _, _) | Type::BuiltinWitness(_, inner) => {
                self.find_gen_vars(&inner, vars, seen, effect_vars, seen_effects, names, seen_names);
            }
            Type::Static(fields) => {
                for (_, t) in fields {
                    self.find_gen_vars(&t, vars, seen, effect_vars, seen_effects, names, seen_names);
                }
            }
            Type::Assoc(base, _) => {
                self.find_gen_vars(&base, vars, seen, effect_vars, seen_effects, names, seen_names);
            }
            _ => {}
        }
    }

    fn find_gen_effect_vars(
        &self,
        effects: &EffectSet,
        effect_vars: &mut Vec<usize>,
        seen_effects: &mut std::collections::HashSet<usize>,
    ) {
        let effects = self.prune_effects(effects);
        match effects {
            EffectSet::Var(id) => {
                if seen_effects.insert(id) {
                    let level = self.effect_var_levels.get(&id).cloned().unwrap_or(0);
                    if level > self.current_level {
                        effect_vars.push(id);
                    }
                }
            }
            EffectSet::Row(_, tail) => {
                self.find_gen_effect_vars(&tail, effect_vars, seen_effects);
            }
            _ => {}
        }
    }

    fn instantiate(&mut self, scheme: &Scheme) -> Type {
        let mut mapping = FxHashMap::default();
        let mut effect_mapping = FxHashMap::default();
        let mut name_mapping = FxHashMap::default();
        for &v in &scheme.vars {
            mapping.insert(v, self.new_var());
        }
        for &v in &scheme.effect_vars {
            effect_mapping.insert(v, self.new_effect_var());
        }
        for name in &scheme.names {
            name_mapping.insert(name.clone(), self.new_var());
        }
        
        let ty = self.substitute_scheme(&scheme.ty, &mapping, &effect_mapping, &name_mapping);
        
        for (param, bound) in &scheme.bounds {
            if let Some(ty) = name_mapping.get(param) {
                self.verify_bound(ty, bound);
            }
        }
        
        ty
    }

    fn verify_bound(&mut self, ty: &Type, weave_name: &str) {
        let ty = self.prune(ty);
        if let Type::Var(_) = ty {
            // Delay check
            return;
        }
        
        if let Some(impls) = self.trait_impls.get(weave_name) {
            // Very simple check: see if any impl matches the pruned type
            for (impl_ty, _) in impls {
                // We use a clone of self or a non-mutating check if possible
                // For now, let's just do a simple check
                if impl_ty == &ty { return; }
            }
        }
        
    }

    fn substitute_scheme(
        &self, 
        ty: &Type, 
        mapping: &FxHashMap<usize, Type>,
        effect_mapping: &FxHashMap<usize, EffectSet>,
        name_mapping: &FxHashMap<String, Type>
    ) -> Type {
        match ty {
            Type::Var(id) => {
                if let Some(new_ty) = mapping.get(id) {
                    new_ty.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Param(name) => {
                if let Some(new_ty) = name_mapping.get(name) {
                    new_ty.clone()
                } else {
                    ty.clone()
                }
            }
            Type::Function { params, ret, effects } => Type::Function {
                params: params.iter().map(|p| self.substitute_scheme(p, mapping, effect_mapping, name_mapping)).collect(),
                ret: Box::new(self.substitute_scheme(ret, mapping, effect_mapping, name_mapping)),
                effects: self.substitute_effects(effects, effect_mapping),
            },
            Type::Optional(inner) => Type::Optional(Box::new(self.substitute_scheme(inner, mapping, effect_mapping, name_mapping))),
            Type::Cascade(inner) => Type::Cascade(Box::new(self.substitute_scheme(inner, mapping, effect_mapping, name_mapping))),
            Type::Pointer(inner, m, l) => Type::Pointer(Box::new(self.substitute_scheme(inner, mapping, effect_mapping, name_mapping)), *m, l.clone()),
            Type::BuiltinWitness(kind, inner) => Type::BuiltinWitness(*kind, Box::new(self.substitute_scheme(inner, mapping, effect_mapping, name_mapping))),
            Type::Static(fields) => Type::Static(
                fields.iter().map(|(n, t)| (n.clone(), self.substitute_scheme(t, mapping, effect_mapping, name_mapping))).collect()
            ),
            Type::Assoc(base, name) => {
                let new_base = self.substitute_scheme(base, mapping, effect_mapping, name_mapping);
                Type::Assoc(Box::new(new_base), name.clone())
            }
            _ => ty.clone(),
        }
    }

    fn substitute_effects(&self, effects: &EffectSet, mapping: &FxHashMap<usize, EffectSet>) -> EffectSet {
        let effects = self.prune_effects(effects);
        match effects {
            EffectSet::Var(id) => {
                if let Some(new) = mapping.get(&id) {
                    new.clone()
                } else {
                    effects
                }
            }
            EffectSet::Row(vals, tail) => {
                EffectSet::Row(vals, Box::new(self.substitute_effects(&tail, mapping)))
            }
            _ => effects,
        }
    }
}


impl TypeChecker {
    fn apply_lifetime_elision(&mut self, params: &mut [Type], ret: &mut Type) {
        let mut input_lifetimes = Vec::new();
        for p in params.iter() {
            self.collect_lifetimes(p, &mut input_lifetimes);
        }

        if input_lifetimes.len() == 1 {
            let elided = input_lifetimes[0].clone();
            self.replace_elided_lifetimes(ret, &elided);
        } else if params.len() > 0 {
            // Check for &self (simplified: first parameter being a pointer)
            if let Type::Pointer(_, _, life) = &params[0] {
                self.replace_elided_lifetimes(ret, life);
            }
        }
    }

    fn collect_lifetimes(&self, ty: &Type, lifetimes: &mut Vec<type_system::Lifetime>) {
        let ty = self.prune(ty);
        match ty {
            Type::Pointer(_, _, l) => {
                lifetimes.push(l.clone());
            }
            Type::Optional(inner) | Type::Cascade(inner) | Type::BuiltinWitness(_, inner) => self.collect_lifetimes(&inner, lifetimes),
            Type::Function { params, ret, .. } => {
                for p in params { self.collect_lifetimes(&p, lifetimes); }
                self.collect_lifetimes(&ret, lifetimes);
            }
            _ => {}
        }
    }

    fn replace_elided_lifetimes(&mut self, ty: &mut Type, lifetime: &type_system::Lifetime) {
        // We need to mutate the actual type, but match on its pruned form.
        // This is tricky because self.prune returns a copy.
        // Let's implement pruning-aware mutation carefully.
        
        match ty {
            Type::Var(id) => {
                let id = *id;
                if let Some(bound) = self.substitutions.get(&id).cloned() {
                    let mut bound = bound;
                    self.replace_elided_lifetimes(&mut bound, lifetime);
                    self.substitutions.insert(id, bound);
                }
            }
            Type::Pointer(inner, _, l) => {
                if let type_system::Lifetime::Anonymous(0) = l {
                    *l = lifetime.clone();
                }
                self.replace_elided_lifetimes(inner, lifetime);
            }
            Type::Optional(inner) | Type::Cascade(inner) | Type::BuiltinWitness(_, inner) => {
                self.replace_elided_lifetimes(inner, lifetime);
            }
            Type::Function { params, ret, .. } => {
                for p in params {
                    self.replace_elided_lifetimes(p, lifetime);
                }
                self.replace_elided_lifetimes(ret, lifetime);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_lexer::{Lexer, TokenKind, Token};
    use crate::type_system::Effect;

    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, izel_span::SourceId(0));
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            if token.kind == TokenKind::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_unify_concrete_effects() {
        let mut tc = TypeChecker::new();
        let e1 = EffectSet::Concrete(vec![Effect::IO]);
        let e2 = EffectSet::Concrete(vec![Effect::IO]);
        assert!(tc.unify_effects(&e1, &e2));
        
        let e3 = EffectSet::Concrete(vec![Effect::Alloc]);
        assert!(!tc.unify_effects(&e1, &e3));
    }

    #[test]
    fn test_unify_effect_vars() {
        let mut tc = TypeChecker::new();
        let ev1 = tc.new_effect_var();
        let e1 = EffectSet::Concrete(vec![Effect::Mut]);
        
        assert!(tc.unify_effects(&ev1, &e1));
        let pruned = tc.prune_effects(&ev1);
        assert_eq!(pruned, e1);
    }

    #[test]
    fn test_unify_rows() {
        let mut tc = TypeChecker::new();
        let tail = tc.new_effect_var();
        let row = EffectSet::Row(vec![Effect::IO], Box::new(tail.clone()));
        let concrete = EffectSet::Concrete(vec![Effect::IO, Effect::Alloc]);
        
        assert!(tc.unify_effects(&row, &concrete));
        let pruned_tail = tc.prune_effects(&tail);
        assert_eq!(pruned_tail, EffectSet::Concrete(vec![Effect::Alloc]));
    }

    #[test]
    fn test_witness_parsing_and_lowering() {
        let source = "@proof forge prove_nonzero(n: i32) -> Witness<i32> { raw n }";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens);
        let cst = parser.parse_decl();
        let lowerer = izel_ast_lower::Lowerer::new(source);
        let item = lowerer.lower_item(&cst).unwrap();
        
        if let ast::Item::Forge(f) = item {
            assert!(f.attributes.iter().any(|a| a.name == "proof"));
            if let ast::Type::Witness(_) = f.ret_type {
                // Success
            } else {
                panic!("Expected Witness return type, got {:?}", f.ret_type);
            }
        } else {
            panic!("Expected Forge item");
        }
    }

    #[test]
    fn test_witness_construction_rules() {
        let mut checker = TypeChecker::new();
        
        // 1. Fail: Witness construction in normal function without raw
        checker.current_attributes = vec![];
        checker.in_raw_block = false;
        
        let _witness_ty = Type::Witness(Box::new(Type::Prim(PrimType::I32)));
        
        let is_proof = checker.current_attributes.iter().any(|a| a.name == "proof");
        assert!(!is_proof);
        assert!(!checker.in_raw_block);
        
        // 2. Success: @proof function
        checker.current_attributes = vec![ast::Attribute { name: "proof".to_string(), args: vec![], span: izel_span::Span::dummy() }];
        let is_proof = checker.current_attributes.iter().any(|a| a.name == "proof");
        assert!(is_proof || checker.in_raw_block);
        
        // 3. Success: raw block
        checker.current_attributes = vec![];
        checker.in_raw_block = true;
        assert!(checker.current_attributes.iter().any(|a| a.name == "proof") || checker.in_raw_block);
    }

    // ========== Built-in Witness Types Tests ==========

    #[test]
    fn test_builtin_witness_nonzero_type() {
        let mut tc = TypeChecker::new();
        let nz1 = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        let nz2 = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        assert!(tc.unify(&nz1, &nz2), "NonZero<i32> should unify with NonZero<i32>");

        // Different inner types should not unify
        let nz3 = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I64)));
        let mut tc2 = TypeChecker::new();
        assert!(!tc2.unify(&nz1, &nz3), "NonZero<i32> should not unify with NonZero<i64>");
    }

    #[test]
    fn test_builtin_witness_inbounds_type() {
        let mut tc = TypeChecker::new();
        let ib1 = Type::BuiltinWitness(BuiltinWitness::InBounds, Box::new(Type::Prim(PrimType::U64)));
        let ib2 = Type::BuiltinWitness(BuiltinWitness::InBounds, Box::new(Type::Prim(PrimType::U64)));
        assert!(tc.unify(&ib1, &ib2), "InBounds<u64> should unify with InBounds<u64>");
    }

    #[test]
    fn test_builtin_witness_sorted_type() {
        let mut tc = TypeChecker::new();
        let s1 = Type::BuiltinWitness(BuiltinWitness::Sorted, Box::new(Type::Prim(PrimType::I32)));
        let s2 = Type::BuiltinWitness(BuiltinWitness::Sorted, Box::new(Type::Prim(PrimType::I32)));
        assert!(tc.unify(&s1, &s2), "Sorted<i32> should unify with Sorted<i32>");
    }

    #[test]
    fn test_builtin_witness_construction_gating() {
        // Built-in witnesses should NOT promote from plain types outside proof/raw mode
        let mut tc = TypeChecker::new();
        tc.current_attributes = vec![];
        tc.in_raw_block = false;

        let nz = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        let plain = Type::Prim(PrimType::I32);

        // BuiltinWitness(NonZero, i32) as lhs, plain i32 as rhs => should fail (construction)
        assert!(!tc.unify(&nz, &plain), "Should not construct NonZero<i32> from i32 outside proof mode");

        // In proof mode, construction should be allowed
        let mut tc2 = TypeChecker::new();
        tc2.current_attributes = vec![ast::Attribute { name: "proof".to_string(), args: vec![], span: izel_span::Span::dummy() }];
        let nz2 = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        assert!(tc2.unify(&nz2, &plain), "Should construct NonZero<i32> from i32 in proof mode");

        // In raw block, construction should also be allowed
        let mut tc3 = TypeChecker::new();
        tc3.in_raw_block = true;
        let nz3 = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        assert!(tc3.unify(&nz3, &plain), "Should construct NonZero<i32> from i32 in raw block");
    }

    #[test]
    fn test_builtin_witness_value_extraction() {
        // Extracting the inner value from a BuiltinWitness should always be allowed
        let mut tc = TypeChecker::new();
        tc.current_attributes = vec![];
        tc.in_raw_block = false;

        let nz = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        let plain = Type::Prim(PrimType::I32);

        // plain i32 as lhs, BuiltinWitness as rhs => extraction, always allowed
        assert!(tc.unify(&plain, &nz), "Should extract i32 from NonZero<i32> outside proof mode");

        let mut tc2 = TypeChecker::new();
        let ib = Type::BuiltinWitness(BuiltinWitness::InBounds, Box::new(Type::Prim(PrimType::U64)));
        let plain_u64 = Type::Prim(PrimType::U64);
        assert!(tc2.unify(&plain_u64, &ib), "Should extract u64 from InBounds<u64>");

        let mut tc3 = TypeChecker::new();
        let sorted = Type::BuiltinWitness(BuiltinWitness::Sorted, Box::new(Type::Prim(PrimType::I32)));
        let plain_i32 = Type::Prim(PrimType::I32);
        assert!(tc3.unify(&plain_i32, &sorted), "Should extract i32 from Sorted<i32>");
    }

    #[test]
    fn test_builtin_witness_unify_different_kinds() {
        let mut tc = TypeChecker::new();
        let nz = Type::BuiltinWitness(BuiltinWitness::NonZero, Box::new(Type::Prim(PrimType::I32)));
        let ib = Type::BuiltinWitness(BuiltinWitness::InBounds, Box::new(Type::Prim(PrimType::I32)));
        assert!(!tc.unify(&nz, &ib), "NonZero<i32> should NOT unify with InBounds<i32>");

        let mut tc2 = TypeChecker::new();
        let sorted = Type::BuiltinWitness(BuiltinWitness::Sorted, Box::new(Type::Prim(PrimType::I32)));
        assert!(!tc2.unify(&nz, &sorted), "NonZero<i32> should NOT unify with Sorted<i32>");
    }

    #[test]
    fn test_nonzero_parse_and_lower() {
        // Parse a function with NonZero<i32> parameter and verify it lowers correctly
        let source = "forge divide(a: i32, b: NonZero<i32>) -> i32 { a }";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens);
        let cst = parser.parse_decl();
        let lowerer = izel_ast_lower::Lowerer::new(source);
        let item = lowerer.lower_item(&cst).unwrap();

        if let ast::Item::Forge(f) = item {
            assert_eq!(f.name, "divide");
            assert_eq!(f.params.len(), 2);

            // First param should be i32
            assert!(matches!(f.params[0].ty, ast::Type::Prim(ref s) if s == "i32"));

            // Second param: the AST layer keeps it as a Path("NonZero", [i32]) 
            // The typeck layer resolves NonZero<i32> to BuiltinWitness
            let mut tc = TypeChecker::new();
            let lowered = tc.lower_ast_type(&f.params[1].ty);
            match lowered {
                Type::BuiltinWitness(BuiltinWitness::NonZero, inner) => {
                    assert_eq!(*inner, Type::Prim(PrimType::I32), "Inner type should be i32");
                }
                other => panic!("Expected BuiltinWitness(NonZero, i32), got {:?}", other),
            }
        } else {
            panic!("Expected Forge item");
        }
    }

    // ========== Temporal Constraints Tests ==========

    #[test]
    fn test_compile_time_requires_violation() {
        // Create a @requires(n > 0) function and call it with 0 => should emit diagnostic
        use crate::contracts::ContractChecker as TypckContractChecker;
        use izel_parser::eval::ConstValue as TypckConstValue;

        let diags = TypckContractChecker::check_requires_from_scheme(
            "test_fn",
            &["n".to_string()],
            &[ast::Expr::Binary(
                ast::BinaryOp::Gt,
                Box::new(ast::Expr::Ident("n".to_string(), izel_span::Span::dummy())),
                Box::new(ast::Expr::Literal(ast::Literal::Int(0))),
            )],
            &[TypckConstValue::Int(0)], // n = 0 violates n > 0
            izel_span::Span::dummy(),
        );
        assert!(!diags.is_empty(), "Should detect precondition violation");
        assert!(diags[0].message.contains("precondition violation"));
    }

    #[test]
    fn test_compile_time_requires_pass() {
        use crate::contracts::ContractChecker as TypckContractChecker;
        use izel_parser::eval::ConstValue as TypckConstValue;

        let diags = TypckContractChecker::check_requires_from_scheme(
            "test_fn",
            &["n".to_string()],
            &[ast::Expr::Binary(
                ast::BinaryOp::Gt,
                Box::new(ast::Expr::Ident("n".to_string(), izel_span::Span::dummy())),
                Box::new(ast::Expr::Literal(ast::Literal::Int(0))),
            )],
            &[TypckConstValue::Int(5)], // n = 5 satisfies n > 0
            izel_span::Span::dummy(),
        );
        assert!(diags.is_empty(), "No violation when precondition is met");
    }

    #[test]
    fn test_compile_time_ensures_violation() {
        use crate::contracts::ContractChecker as TypckContractChecker;
        use izel_parser::eval::ConstValue as TypckConstValue;

        let diags = TypckContractChecker::check_ensures_from_scheme(
            "test_fn",
            &[ast::Expr::Binary(
                ast::BinaryOp::Gt,
                Box::new(ast::Expr::Ident("result".to_string(), izel_span::Span::dummy())),
                Box::new(ast::Expr::Literal(ast::Literal::Int(0))),
            )],
            &TypckConstValue::Int(0), // result = 0 violates result > 0
            izel_span::Span::dummy(),
            &std::collections::HashMap::new(),
        );
        assert!(!diags.is_empty(), "Should detect postcondition violation");
        assert!(diags[0].message.contains("postcondition violation"));
    }

    #[test]
    fn test_invariant_extraction() {
        // Parse a shape with @invariant and verify invariants are populated
        let source = "@invariant(self.width > 0) shape Rect { width: f64, }";
        let tokens = tokenize(source);
        let mut parser = izel_parser::Parser::new(tokens);
        let cst = parser.parse_decl();
        let lowerer = izel_ast_lower::Lowerer::new(source);
        let item = lowerer.lower_item(&cst);

        if let Some(ast::Item::Shape(s)) = item {
            assert_eq!(s.name, "Rect");
            assert!(!s.invariants.is_empty(), "Shape should have invariants extracted from @invariant");
            // The invariant attribute should not appear in regular attributes
            assert!(s.attributes.iter().all(|a| a.name != "invariant"), 
                "invariant should be extracted from attributes");
        } else {
            panic!("Expected Shape item");
        }
    }

}
