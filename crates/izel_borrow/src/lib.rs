//! Ownership and Borrow Checking for Izel.

use izel_mir::{MirBody, Local, Instruction, Terminator, Operand, Place, Rvalue};
use izel_typeck::Type;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct BorrowChecker {
    pub errors: Vec<String>,
    pub borrows: Vec<ActiveBorrow>,
}

pub struct ActiveBorrow {
    pub local: Local,
    pub is_mut: bool,
    pub region: FxHashSet<izel_mir::BlockId>, // Simple NLL region as a set of blocks
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            borrows: Vec::new(),
        }
    }

    pub fn check(&mut self, mir: &MirBody) -> Result<(), Vec<String>> {
        let liveness = LivenessAnalysis::compute(mir);
        let active_borrows = self.infer_borrow_regions(mir, &liveness);
        self.check_all(mir, &active_borrows);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn infer_borrow_regions(&self, mir: &MirBody, liveness: &LivenessAnalysis) -> FxHashMap<Local, Vec<ActiveBorrow>> {
        let mut active_borrows: FxHashMap<Local, Vec<ActiveBorrow>> = FxHashMap::default();
        for node in mir.blocks.node_indices() {
             let block = &mir.blocks[node];
             for instr in &block.instructions {
                 if let Instruction::Assign(place, Rvalue::Ref(borrowed_place, is_mut)) = instr {
                     let mut region = FxHashSet::default();
                     region.insert(node);
                     for (bn, live_set) in &liveness.live_out {
                         if live_set.contains(&place.local) {
                             region.insert(*bn);
                         }
                     }
                     active_borrows.entry(borrowed_place.local).or_default().push(ActiveBorrow {
                         local: borrowed_place.local,
                         is_mut: *is_mut,
                         region,
                     });
                 }
             }
        }
        active_borrows
    }

    fn check_all(&mut self, mir: &MirBody, active_borrows: &FxHashMap<Local, Vec<ActiveBorrow>>) {
        let mut initialized = FxHashSet::default();
        
        for node in mir.blocks.node_indices() {
            let block = &mir.blocks[node];
            
            // Check for borrow clashes in this block (pre-instruction state)
            for (borrowed_local, borrows) in active_borrows {
                let mut has_mut = false;
                let mut shared_count = 0;
                for b in borrows {
                    if b.region.contains(&node) {
                        if b.is_mut { has_mut = true; } else { shared_count += 1; }
                    }
                }
                if has_mut && shared_count > 0 {
                    self.errors.push(format!("Cannot borrow {} as shared while it is already borrowed as mutable", mir.locals[borrowed_local.0].name));
                }
                if has_mut && borrows.iter().filter(|b| b.is_mut && b.region.contains(&node)).count() > 1 {
                    self.errors.push(format!("Cannot borrow {} as mutable more than once at a time", mir.locals[borrowed_local.0].name));
                }
            }

            for instr in &block.instructions {
                match instr {
                    Instruction::Assign(place, rvalue) => {
                        self.check_rvalue(rvalue, &mut initialized, mir, active_borrows, node);
                        initialized.insert(place.local);
                    }
                    Instruction::Call(place, _, args) => {
                        for arg in args {
                            self.check_operand(arg, &mut initialized, mir, active_borrows, node);
                        }
                        initialized.insert(place.local);
                    }
                    _ => {}
                }
            }
            if let Some(term) = &block.terminator {
                match term {
                    Terminator::SwitchInt(op, _, _) => {
                         self.check_operand(op, &mut initialized, mir, active_borrows, node);
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_rvalue(&mut self, rvalue: &Rvalue, initialized: &mut FxHashSet<Local>, mir: &MirBody, active_borrows: &FxHashMap<Local, Vec<ActiveBorrow>>, block: izel_mir::BlockId) {
        match rvalue {
            Rvalue::Use(op) => self.check_operand(op, initialized, mir, active_borrows, block),
            Rvalue::BinaryOp(_, l, r) => {
                self.check_operand(l, initialized, mir, active_borrows, block);
                self.check_operand(r, initialized, mir, active_borrows, block);
            }
            Rvalue::UnaryOp(_, op) => self.check_operand(op, initialized, mir, active_borrows, block),
            Rvalue::Ref(place, _) => {
                if !initialized.contains(&place.local) {
                    self.errors.push(format!("Cannot borrow uninitialized or moved variable: {}", mir.locals[place.local.0].name));
                }
            }
        }
    }

    fn check_operand(&mut self, op: &Operand, initialized: &mut FxHashSet<Local>, mir: &MirBody, active_borrows: &FxHashMap<Local, Vec<ActiveBorrow>>, block: izel_mir::BlockId) {
        match op {
            Operand::Move(place) | Operand::Copy(place) => {
                if !initialized.contains(&place.local) {
                    self.errors.push(format!("Use of uninitialized or moved variable: {}", mir.locals[place.local.0].name));
                }
                
                // Check if moving borrowed data
                if let Operand::Move(_) = op {
                    if let Some(borrows) = active_borrows.get(&place.local) {
                        if borrows.iter().any(|b| b.region.contains(&block)) {
                            self.errors.push(format!("Cannot move {} because it is currently borrowed", mir.locals[place.local.0].name));
                        }
                    }
                }

                let ty = &mir.locals[place.local.0].ty;
                if !self.is_type_copyable(ty) {
                    if let Operand::Move(_) = op {
                        initialized.remove(&place.local);
                    } else if let Operand::Copy(_) = op {
                        self.errors.push(format!("Cannot copy non-copyable type {:?} for variable {}", ty, mir.locals[place.local.0].name));
                    }
                }
            }
            _ => {}
        }
    }

    fn is_type_copyable(&self, ty: &Type) -> bool {
        match ty {
            Type::Prim(_) => true,
            Type::Pointer(_, is_mut, _) => !*is_mut, // Shared pointers are copyable, mutable ones are NOT
            Type::Error => true, // Treat errors as copyable to avoid cascading errors
            _ => false,
        }
    }
}

pub struct LivenessAnalysis {
    pub live_in: FxHashMap<izel_mir::BlockId, FxHashSet<Local>>,
    pub live_out: FxHashMap<izel_mir::BlockId, FxHashSet<Local>>,
}

impl LivenessAnalysis {
    pub fn compute(mir: &MirBody) -> Self {
        let mut live_in: FxHashMap<_, FxHashSet<Local>> = FxHashMap::default();
        let mut live_out: FxHashMap<_, FxHashSet<Local>> = FxHashMap::default();

        let mut changed = true;
        while changed {
            changed = false;
            
            for node in mir.blocks.node_indices() {
                // LiveOut[n] = U { LiveIn[s] | s in successors(n) }
                let mut new_live_out = FxHashSet::default();
                for successor in mir.blocks.neighbors(node) {
                    if let Some(s_live_in) = live_in.get(&successor) {
                        new_live_out.extend(s_live_in.iter().cloned());
                    }
                }
                
                if live_out.get(&node) != Some(&new_live_out) {
                    live_out.insert(node, new_live_out.clone());
                    changed = true;
                }

                // LiveIn[n] = Use[n] U (LiveOut[n] - Def[n])
                let block = &mir.blocks[node];
                let (uses, defs) = Self::get_block_uses_defs(block);
                
                let mut new_live_in = uses;
                for local in &new_live_out {
                    if !defs.contains(local) {
                        new_live_in.insert(*local);
                    }
                }

                if live_in.get(&node) != Some(&new_live_in) {
                    live_in.insert(node, new_live_in);
                    changed = true;
                }
            }
        }

        Self { live_in, live_out }
    }

    fn get_block_uses_defs(block: &izel_mir::BasicBlock) -> (FxHashSet<Local>, FxHashSet<Local>) {
        let mut uses = FxHashSet::default();
        let mut defs = FxHashSet::default();

        for instr in &block.instructions {
            match instr {
                Instruction::Assign(place, rvalue) => {
                    Self::get_rvalue_uses(rvalue, &mut uses, &defs);
                    defs.insert(place.local);
                }
                Instruction::Call(place, _, args) => {
                    for arg in args {
                        Self::get_operand_use(arg, &mut uses, &defs);
                    }
                    defs.insert(place.local);
                }
                _ => {}
            }
        }
        
        if let Some(term) = &block.terminator {
            match term {
                Terminator::SwitchInt(op, _, _) => {
                    Self::get_operand_use(op, &mut uses, &defs);
                }
                _ => {}
            }
        }

        (uses, defs)
    }

    fn get_rvalue_uses(rv: &Rvalue, uses: &mut FxHashSet<Local>, defs: &FxHashSet<Local>) {
        match rv {
            Rvalue::Use(op) => Self::get_operand_use(op, uses, defs),
            Rvalue::BinaryOp(_, l, r) => {
                Self::get_operand_use(l, uses, defs);
                Self::get_operand_use(r, uses, defs);
            }
            Rvalue::UnaryOp(_, op) => Self::get_operand_use(op, uses, defs),
            Rvalue::Ref(p, _) => {
                if !defs.contains(&p.local) {
                    uses.insert(p.local);
                }
            }
        }
    }

    fn get_operand_use(op: &Operand, uses: &mut FxHashSet<Local>, defs: &FxHashSet<Local>) {
        match op {
            Operand::Copy(p) | Operand::Move(p) => {
                if !defs.contains(&p.local) {
                    uses.insert(p.local);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_mir::{Instruction, Place, Local, Rvalue, Operand, MirBody, LocalData};
    use izel_typeck::type_system::Type;

    #[test]
    fn test_move_error() {
        let mut mir = MirBody::new();
        let local0 = Local(0);
        // Using a non-copyable type (Adt)
        mir.locals.push(LocalData { name: "x".into(), ty: Type::Adt(izel_resolve::DefId(0)) });
        let local1 = Local(1);
        mir.locals.push(LocalData { name: "y".into(), ty: Type::Adt(izel_resolve::DefId(0)) });
        let local2 = Local(2);
        mir.locals.push(LocalData { name: "z".into(), ty: Type::Adt(izel_resolve::DefId(0)) });

        let block = mir.blocks.node_weight_mut(mir.entry).unwrap();
        // x = 1
        block.instructions.push(Instruction::Assign(
            Place { local: local0 },
            Rvalue::Use(Operand::Constant(izel_mir::Constant::Int(1)))
        ));
        // y = move x
        block.instructions.push(Instruction::Assign(
            Place { local: local1 },
            Rvalue::Use(Operand::Move(Place { local: local0 }))
        ));
        // z = move x (ERROR)
        block.instructions.push(Instruction::Assign(
            Place { local: Local(2) },
            Rvalue::Use(Operand::Move(Place { local: local0 }))
        ));

        let mut bc = BorrowChecker::new();
        let res = bc.check(&mir);
        assert!(res.is_err());
        assert!(res.unwrap_err()[0].contains("Use of uninitialized or moved variable: x"));
    }

    #[test]
    fn test_borrow_clash() {
        let mut mir = MirBody::new();
        let x = Local(0);
        mir.locals.push(LocalData { name: "x".into(), ty: Type::Adt(izel_resolve::DefId(0)) });
        let y = Local(1);
        mir.locals.push(LocalData { name: "y".into(), ty: Type::Pointer(Box::new(Type::Adt(izel_resolve::DefId(0))), true, izel_typeck::type_system::Lifetime::Anonymous(0)) });
        let z = Local(2);
        mir.locals.push(LocalData { name: "z".into(), ty: Type::Pointer(Box::new(Type::Adt(izel_resolve::DefId(0))), false, izel_typeck::type_system::Lifetime::Anonymous(0)) });

        let block = mir.blocks.node_weight_mut(mir.entry).unwrap();
        // x = 1
        block.instructions.push(Instruction::Assign(
            Place { local: x },
            Rvalue::Use(Operand::Constant(izel_mir::Constant::Int(1)))
        ));
        // y = &~x (mutable borrow)
        block.instructions.push(Instruction::Assign(
            Place { local: y },
            Rvalue::Ref(Place { local: x }, true)
        ));
        // z = &x (immutable borrow - CLASH)
        block.instructions.push(Instruction::Assign(
            Place { local: z },
            Rvalue::Ref(Place { local: x }, false)
        ));

        let mut bc = BorrowChecker::new();
        let res = bc.check(&mir);
        assert!(res.is_err());
        assert!(res.unwrap_err().iter().any(|e| e.contains("already borrowed as mutable")));
    }

    #[test]
    fn test_move_while_borrowed() {
        let mut mir = MirBody::new();
        let x = Local(0);
        mir.locals.push(LocalData { name: "x".into(), ty: Type::Adt(izel_resolve::DefId(0)) });
        let y = Local(1);
        mir.locals.push(LocalData { name: "y".into(), ty: Type::Pointer(Box::new(Type::Adt(izel_resolve::DefId(0))), false, izel_typeck::type_system::Lifetime::Anonymous(0)) });

        let block = mir.blocks.node_weight_mut(mir.entry).unwrap();
        // x = ADT
        block.instructions.push(Instruction::Assign(
            Place { local: x },
            Rvalue::Use(Operand::Constant(izel_mir::Constant::Int(1)))
        ));
        // y = &x (borrow)
        block.instructions.push(Instruction::Assign(
            Place { local: y },
            Rvalue::Ref(Place { local: x }, false)
        ));
        // z = move x (ERROR: x is borrowed)
        block.instructions.push(Instruction::Assign(
            Place { local: Local(2) },
            Rvalue::Use(Operand::Move(Place { local: x }))
        ));

        let mut bc = BorrowChecker::new();
        let res = bc.check(&mir);
        assert!(res.is_err());
        assert!(res.unwrap_err().iter().any(|e| e.contains("Cannot move x because it is currently borrowed")));
    }
}
