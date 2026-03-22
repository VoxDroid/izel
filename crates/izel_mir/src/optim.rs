use crate::*;
use std::collections::HashSet;

pub struct Dce;

impl Dce {
    pub fn run(body: &mut MirBody) {
        let mut changed = true;
        while changed {
            changed = false;

            // 1. Identify used locals
            let mut used_locals = HashSet::new();
            for block in body.blocks.node_weights() {
                for instr in &block.instructions {
                    self::collect_used_locals_instr(instr, &mut used_locals);
                }
                if let Some(term) = &block.terminator {
                    self::collect_used_locals_term(term, &mut used_locals);
                }
            }

            // 2. Sweep unused assignments
            for block in body.blocks.node_weights_mut() {
                let mut i = 0;
                while i < block.instructions.len() {
                    let mut remove = false;
                    if let Instruction::Assign(local, _) = &block.instructions[i] {
                        if !used_locals.contains(local) {
                            remove = true;
                        }
                    } else if let Instruction::Phi(local, _) = &block.instructions[i] {
                        if !used_locals.contains(local) {
                            remove = true;
                        }
                    }

                    if remove {
                        block.instructions.remove(i);
                        changed = true;
                    } else {
                        i += 1;
                    }
                }
            }
        }

        // 3. Remove unreachable blocks
        let mut reachable = HashSet::new();
        let mut stack = vec![body.entry];
        while let Some(node) = stack.pop() {
            if reachable.insert(node) {
                for succ in body.blocks.neighbors(node) {
                    stack.push(succ);
                }
            }
        }

        for idx in body.blocks.node_indices().collect::<Vec<_>>() {
            if !reachable.contains(&idx) {
                body.blocks[idx].instructions.clear();
                body.blocks[idx].terminator = None;
            }
        }
    }
}

pub struct PipelineFusion;

impl PipelineFusion {
    pub fn run(_body: &mut MirBody) {
        // Find consecutive higher-order function calls and fuse them
    }
}

pub struct Licm;

impl Licm {
    pub fn run(_body: &mut MirBody) {
        // Move loop-invariant instructions to pre-headers
    }
}

fn collect_used_locals_instr(instr: &Instruction, used: &mut HashSet<Local>) {
    match instr {
        Instruction::Assign(_, rv) => collect_used_locals_rvalue(rv, used),
        Instruction::Phi(_, operands) => {
            for (_, local) in operands {
                used.insert(*local);
            }
        }
        Instruction::Call(_, _, ops) => {
            for op in ops {
                collect_used_locals_operand(op, used);
            }
        }
        Instruction::Assert(op, _) => collect_used_locals_operand(op, used),
        _ => {}
    }
}

fn collect_used_locals_term(term: &Terminator, used: &mut HashSet<Local>) {
    match term {
        Terminator::Return(op) => {
            if let Some(o) = op {
                collect_used_locals_operand(o, used);
            }
        }
        Terminator::Goto(_) => {}
        Terminator::SwitchInt(op, _, _) => collect_used_locals_operand(op, used),
        Terminator::Abort => {}
    }
}

fn collect_used_locals_rvalue(rv: &Rvalue, used: &mut HashSet<Local>) {
    match rv {
        Rvalue::Use(op) => collect_used_locals_operand(op, used),
        Rvalue::BinaryOp(_, l, r) => {
            collect_used_locals_operand(l, used);
            collect_used_locals_operand(r, used);
        }
        Rvalue::UnaryOp(_, inner) => collect_used_locals_operand(inner, used),
        Rvalue::Ref(local, _) => {
            used.insert(*local);
        }
    }
}

fn collect_used_locals_operand(op: &Operand, used: &mut HashSet<Local>) {
    match op {
        Operand::Copy(l) | Operand::Move(l) => {
            used.insert(*l);
        }
        _ => {}
    }
}
