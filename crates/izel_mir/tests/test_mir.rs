use izel_mir::optim::{Dce, Licm, PipelineFusion};
use izel_mir::{BasicBlock, ControlFlow};
use izel_mir::{
    BinOp, Constant, Instruction, Local, LocalData, MirBody, Operand, Rvalue, Terminator, UnOp,
};
use izel_typeck::type_system::{PrimType, Type};

#[test]
fn mir_body_new_creates_entry_block() {
    let body = MirBody::new();

    assert_eq!(body.arg_count, 0);
    assert!(body.locals.is_empty());
    assert!(body.blocks.node_weight(body.entry).is_some());
}

#[test]
fn mir_body_allows_instruction_and_terminator_insertion() {
    let mut body = MirBody::new();
    body.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Prim(PrimType::I32),
    });

    let block = body.blocks.node_weight_mut(body.entry).unwrap();
    block.instructions.push(Instruction::Assign(
        Local(0),
        Rvalue::Use(Operand::Constant(Constant::Int(7))),
    ));
    block.terminator = Some(Terminator::Return(Some(Operand::Copy(Local(0)))));

    assert_eq!(block.instructions.len(), 1);
    assert!(matches!(
        block.terminator,
        Some(Terminator::Return(Some(Operand::Copy(Local(0)))))
    ));
}

#[test]
fn dce_removes_unused_assignments_but_keeps_used_dataflow() {
    let mut body = MirBody::new();
    body.locals.push(LocalData {
        name: "a".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "b".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "unused".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "phi".to_string(),
        ty: Type::Prim(PrimType::I32),
    });

    let entry = body.entry;
    let block = body.blocks.node_weight_mut(entry).unwrap();
    block.instructions.push(Instruction::Assign(
        Local(0),
        Rvalue::Use(Operand::Constant(Constant::Int(1))),
    ));
    block.instructions.push(Instruction::Assign(
        Local(1),
        Rvalue::Use(Operand::Copy(Local(0))),
    ));
    block.instructions.push(Instruction::Assign(
        Local(2),
        Rvalue::Use(Operand::Constant(Constant::Int(99))),
    ));
    block
        .instructions
        .push(Instruction::Phi(Local(3), vec![(entry, Local(1))]));
    block.terminator = Some(Terminator::Return(Some(Operand::Copy(Local(3)))));

    Dce::run(&mut body);

    let block = body.blocks.node_weight(entry).unwrap();
    assert_eq!(block.instructions.len(), 3);
    assert!(!block
        .instructions
        .iter()
        .any(|instr| matches!(instr, Instruction::Assign(Local(2), _))));
}

#[test]
fn dce_handles_call_assert_and_switchint_usage() {
    let mut body = MirBody::new();
    body.locals.push(LocalData {
        name: "arg".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "res".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "unused".to_string(),
        ty: Type::Prim(PrimType::I32),
    });

    let entry = body.entry;
    let block = body.blocks.node_weight_mut(entry).unwrap();
    block.instructions.push(Instruction::Assign(
        Local(0),
        Rvalue::Use(Operand::Constant(Constant::Int(5))),
    ));
    block.instructions.push(Instruction::Call(
        Some(Local(1)),
        "id".to_string(),
        vec![Operand::Copy(Local(0))],
    ));
    block.instructions.push(Instruction::Assert(
        Operand::Copy(Local(1)),
        "expected non-zero".to_string(),
    ));
    block.instructions.push(Instruction::Assign(
        Local(2),
        Rvalue::Use(Operand::Constant(Constant::Int(42))),
    ));
    block.terminator = Some(Terminator::SwitchInt(
        Operand::Copy(Local(1)),
        vec![(0, entry)],
        entry,
    ));

    Dce::run(&mut body);

    let block = body.blocks.node_weight(entry).unwrap();
    assert!(!block
        .instructions
        .iter()
        .any(|instr| matches!(instr, Instruction::Assign(Local(2), _))));
}

#[test]
fn dce_clears_unreachable_blocks() {
    let mut body = MirBody::new();
    body.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    body.locals.push(LocalData {
        name: "y".to_string(),
        ty: Type::Prim(PrimType::I32),
    });

    let entry = body.entry;
    let reachable = body.blocks.add_node(BasicBlock {
        instructions: Vec::new(),
        terminator: None,
    });
    let unreachable = body.blocks.add_node(BasicBlock {
        instructions: Vec::new(),
        terminator: None,
    });
    body.blocks
        .add_edge(entry, reachable, ControlFlow::Unconditional);
    body.blocks
        .add_edge(reachable, entry, ControlFlow::Unconditional);

    let entry_block = body.blocks.node_weight_mut(entry).unwrap();
    entry_block.instructions.push(Instruction::Assign(
        Local(0),
        Rvalue::Use(Operand::Constant(Constant::Int(10))),
    ));
    entry_block.terminator = Some(Terminator::Goto(reachable));

    let reachable_block = body.blocks.node_weight_mut(reachable).unwrap();
    reachable_block.instructions.push(Instruction::Assign(
        Local(1),
        Rvalue::Use(Operand::Copy(Local(0))),
    ));
    reachable_block.terminator = Some(Terminator::Return(Some(Operand::Copy(Local(1)))));

    let dead_block = body.blocks.node_weight_mut(unreachable).unwrap();
    dead_block.instructions.push(Instruction::Assign(
        Local(1),
        Rvalue::Use(Operand::Constant(Constant::Int(999))),
    ));
    dead_block.terminator = Some(Terminator::Abort);

    Dce::run(&mut body);

    let dead_block = body.blocks.node_weight(unreachable).unwrap();
    assert!(dead_block.instructions.is_empty());
    assert!(dead_block.terminator.is_none());
}

#[test]
fn optimization_pass_helpers_are_callable() {
    let mut body = MirBody::new();
    PipelineFusion::run(&mut body);
    Licm::run(&mut body);

    assert!(body.blocks.node_weight(body.entry).is_some());
}

#[test]
fn dce_covers_binary_unary_ref_abort_and_unused_phi_paths() {
    let mut body = MirBody::new();
    for idx in 0..5 {
        body.locals.push(LocalData {
            name: format!("l{idx}"),
            ty: Type::Prim(PrimType::I32),
        });
    }

    let entry = body.entry;
    let block = body.blocks.node_weight_mut(entry).unwrap();
    block.instructions.push(Instruction::Assign(
        Local(0),
        Rvalue::Use(Operand::Constant(Constant::Int(1))),
    ));
    block.instructions.push(Instruction::Assign(
        Local(1),
        Rvalue::Binary(
            BinOp::Add,
            Operand::Copy(Local(0)),
            Operand::Constant(Constant::Int(2)),
        ),
    ));
    block.instructions.push(Instruction::StorageLive(Local(0)));
    block.instructions.push(Instruction::StorageDead(Local(0)));
    block.instructions.push(Instruction::Assign(
        Local(2),
        Rvalue::Unary(UnOp::Neg, Operand::Copy(Local(1))),
    ));
    block
        .instructions
        .push(Instruction::Assign(Local(3), Rvalue::Ref(Local(2), false)));
    block
        .instructions
        .push(Instruction::Phi(Local(4), vec![(entry, Local(2))]));
    block.terminator = Some(Terminator::Abort);

    Dce::run(&mut body);

    let block = body.blocks.node_weight(entry).unwrap();
    assert!(matches!(block.terminator, Some(Terminator::Abort)));
    assert!(block
        .instructions
        .iter()
        .any(|instr| matches!(instr, Instruction::StorageLive(Local(0)))));
    assert!(!block
        .instructions
        .iter()
        .any(|instr| matches!(instr, Instruction::Phi(Local(4), _))));
}
