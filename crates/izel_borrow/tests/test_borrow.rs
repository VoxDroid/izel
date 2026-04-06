use izel_borrow::BorrowChecker;
use izel_mir::{Constant, Instruction, Local, LocalData, MirBody, Operand, Rvalue};
use izel_typeck::type_system::{PrimType, Type};

#[test]
fn borrow_checker_accepts_copyable_value_flow() {
    let mut mir = MirBody::new();
    let x = Local(0);
    let y = Local(1);

    mir.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Prim(PrimType::I32),
    });
    mir.locals.push(LocalData {
        name: "y".to_string(),
        ty: Type::Prim(PrimType::I32),
    });

    let block = mir.blocks.node_weight_mut(mir.entry).unwrap();
    block.instructions.push(Instruction::Assign(
        x,
        Rvalue::Use(Operand::Constant(Constant::Int(10))),
    ));
    block
        .instructions
        .push(Instruction::Assign(y, Rvalue::Use(Operand::Copy(x))));

    let mut checker = BorrowChecker::new();
    assert!(checker.check(&mir).is_ok());
}

#[test]
fn borrow_checker_reports_use_after_move_for_non_copy_type() {
    let mut mir = MirBody::new();
    let x = Local(0);
    let y = Local(1);
    let z = Local(2);

    mir.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Adt(izel_resolve::DefId(1)),
    });
    mir.locals.push(LocalData {
        name: "y".to_string(),
        ty: Type::Adt(izel_resolve::DefId(1)),
    });
    mir.locals.push(LocalData {
        name: "z".to_string(),
        ty: Type::Adt(izel_resolve::DefId(1)),
    });

    let block = mir.blocks.node_weight_mut(mir.entry).unwrap();
    block.instructions.push(Instruction::Assign(
        x,
        Rvalue::Use(Operand::Constant(Constant::Int(1))),
    ));
    block
        .instructions
        .push(Instruction::Assign(y, Rvalue::Use(Operand::Move(x))));
    block
        .instructions
        .push(Instruction::Assign(z, Rvalue::Use(Operand::Move(x))));

    let mut checker = BorrowChecker::new();
    let errors = checker
        .check(&mir)
        .expect_err("expected move-after-move error");
    assert!(errors
        .iter()
        .any(|e| e.contains("Use of uninitialized or moved variable: x")));
}

#[test]
fn reborrow_after_last_use_in_later_block_is_allowed() {
    let mut mir = MirBody::new();
    let x = Local(0);
    let mut_ref = Local(1);
    let shared_ref = Local(2);
    let sink1 = Local(3);
    let sink2 = Local(4);

    mir.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Adt(izel_resolve::DefId(7)),
    });
    mir.locals.push(LocalData {
        name: "mx".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(7))),
            true,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });
    mir.locals.push(LocalData {
        name: "sx".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(7))),
            false,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });
    mir.locals.push(LocalData {
        name: "sink1".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(7))),
            true,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });
    mir.locals.push(LocalData {
        name: "sink2".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(7))),
            false,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });

    let block1 = mir.entry;
    let block2 = mir.blocks.add_node(izel_mir::BasicBlock {
        instructions: Vec::new(),
        terminator: None,
    });
    mir.blocks
        .add_edge(block1, block2, izel_mir::ControlFlow::Unconditional);

    let b1 = mir.blocks.node_weight_mut(block1).unwrap();
    b1.instructions.push(Instruction::Assign(
        x,
        Rvalue::Use(Operand::Constant(Constant::Int(1))),
    ));
    b1.instructions
        .push(Instruction::Assign(mut_ref, Rvalue::Ref(x, true)));
    b1.instructions.push(Instruction::Assign(
        sink1,
        Rvalue::Use(Operand::Move(mut_ref)),
    ));
    b1.terminator = Some(izel_mir::Terminator::Goto(block2));

    let b2 = mir.blocks.node_weight_mut(block2).unwrap();
    b2.instructions
        .push(Instruction::Assign(shared_ref, Rvalue::Ref(x, false)));
    b2.instructions.push(Instruction::Assign(
        sink2,
        Rvalue::Use(Operand::Copy(shared_ref)),
    ));

    let mut checker = BorrowChecker::new();
    let result = checker.check(&mir);
    assert!(
        result.is_ok(),
        "reborrow after last use should be accepted, got: {:?}",
        result.err()
    );
}

#[test]
fn borrow_checker_reports_double_mutable_borrow_in_same_region() {
    let mut mir = MirBody::new();
    let x = Local(0);
    let m1 = Local(1);
    let m2 = Local(2);

    mir.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Adt(izel_resolve::DefId(10)),
    });
    mir.locals.push(LocalData {
        name: "m1".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(10))),
            true,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });
    mir.locals.push(LocalData {
        name: "m2".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(10))),
            true,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });

    let bb = mir.blocks.node_weight_mut(mir.entry).unwrap();
    bb.instructions.push(Instruction::Assign(
        x,
        Rvalue::Use(Operand::Constant(Constant::Int(1))),
    ));
    bb.instructions
        .push(Instruction::Assign(m1, Rvalue::Ref(x, true)));
    bb.instructions
        .push(Instruction::Assign(m2, Rvalue::Ref(x, true)));

    let mut checker = BorrowChecker::default();
    let errors = checker
        .check(&mir)
        .expect_err("double mutable borrow should be rejected");
    assert!(errors.iter().any(|e| e.contains("mutable more than once")));
}

#[test]
fn borrow_checker_covers_zone_call_tracking_and_uninitialized_ref_paths() {
    let mut mir = MirBody::new();
    let x = Local(0);
    let out = Local(1);

    mir.locals.push(LocalData {
        name: "x".to_string(),
        ty: Type::Adt(izel_resolve::DefId(11)),
    });
    mir.locals.push(LocalData {
        name: "out".to_string(),
        ty: Type::Pointer(
            Box::new(Type::Adt(izel_resolve::DefId(11))),
            false,
            izel_typeck::type_system::Lifetime::Anonymous(0),
        ),
    });

    let bb = mir.blocks.node_weight_mut(mir.entry).unwrap();
    bb.instructions
        .push(Instruction::ZoneEnter("arena".to_string()));
    bb.instructions.push(Instruction::Call(
        Some(out),
        "alloc_like".to_string(),
        vec![Operand::Constant(Constant::Int(1))],
    ));
    bb.instructions.push(Instruction::Phi(Local(0), vec![]));
    bb.instructions
        .push(Instruction::Assign(Local(1), Rvalue::Ref(x, false)));
    bb.instructions
        .push(Instruction::ZoneExit("arena".to_string()));

    let mut checker = BorrowChecker::new();
    let errors = checker
        .check(&mir)
        .expect_err("uninitialized ref should be reported");
    assert!(errors
        .iter()
        .any(|e| e.contains("Cannot borrow uninitialized or moved variable: x")));
}

#[test]
fn borrow_checker_zone_enter_exit_without_payload_is_valid() {
    let mut mir = MirBody::new();
    let bb = mir.blocks.node_weight_mut(mir.entry).unwrap();
    bb.instructions
        .push(Instruction::ZoneEnter("arena".to_string()));
    bb.instructions
        .push(Instruction::ZoneExit("arena".to_string()));

    let mut checker = BorrowChecker::new();
    assert!(checker.check(&mir).is_ok());
}
