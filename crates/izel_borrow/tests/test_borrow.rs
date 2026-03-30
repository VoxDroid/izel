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
