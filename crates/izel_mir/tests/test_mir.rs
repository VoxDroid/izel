use izel_mir::{Constant, Instruction, Local, LocalData, MirBody, Operand, Rvalue, Terminator};
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
