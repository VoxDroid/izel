//! LLVM code generation for Izel.

use anyhow::{anyhow, Result};
use inkwell::basic_block::BasicBlock as LlvmBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use izel_mir::{
    BinOp, BlockId, Constant, Instruction, Local, MirBody, Operand, Rvalue, Terminator, UnOp,
};
use izel_parser::ast;
use izel_typeck::type_system::{PrimType, Type};
use std::collections::HashMap;

pub struct Codegen<'ctx, 'a> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub source: &'a str,
    pub variables: HashMap<String, PointerValue<'ctx>>,
}

pub struct MirCodegen<'ctx, 'a> {
    pub context: &'ctx Context,
    pub module: &'a Module<'ctx>,
    pub builder: &'a Builder<'ctx>,
    pub locals: HashMap<Local, PointerValue<'ctx>>,
    pub blocks: HashMap<BlockId, LlvmBasicBlock<'ctx>>,
}

impl<'ctx, 'a> MirCodegen<'ctx, 'a> {
    pub fn new(
        context: &'ctx Context,
        module: &'a Module<'ctx>,
        builder: &'a Builder<'ctx>,
    ) -> Self {
        Self {
            context,
            module,
            builder,
            locals: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    pub fn gen_mir_body(&mut self, function: FunctionValue<'ctx>, body: &MirBody) -> Result<()> {
        // 1. Pre-create all LLVM basic blocks
        for node in body.blocks.node_indices() {
            let label = format!("bb{}", node.index());
            let bb = self.context.append_basic_block(function, &label);
            self.blocks.insert(node, bb);
        }

        // 2. Allocate all locals in the entry block
        let entry_bb = self.blocks[&body.entry];
        self.builder.position_at_end(entry_bb);
        for (i, local_data) in body.locals.iter().enumerate() {
            let ty = self.llvm_type(&local_data.ty)?;
            let ptr = self.builder.build_alloca(ty, &local_data.name)?;
            self.locals.insert(Local(i), ptr);
        }

        // 2.1 Store function parameters into locals (first N locals)
        for (i, llvm_param) in function.get_param_iter().enumerate() {
            if let Some(ptr) = self.locals.get(&Local(i)) {
                self.builder.build_store(*ptr, llvm_param)?;
            }
        }

        // 3. Lower each block
        for node in body.blocks.node_indices() {
            let bb = self.blocks[&node];
            self.builder.position_at_end(bb);
            let mir_bb = &body.blocks[node];

            for inst in &mir_bb.instructions {
                self.gen_instruction(inst, body)?;
            }

            if let Some(term) = &mir_bb.terminator {
                self.gen_terminator(term, body)?;
            } else {
                // If no terminator, build an implicit return or unreachable
                self.builder.build_unreachable()?;
            }
        }

        Ok(())
    }

    fn gen_instruction(&mut self, inst: &Instruction, body: &MirBody) -> Result<()> {
        match inst {
            Instruction::Assign(local, rvalue) => {
                let val = self.gen_rvalue(rvalue, body)?;
                let ptr = self.locals[local];
                self.builder.build_store(ptr, val)?;
            }
            Instruction::Call(dest, name, args) => {
                let function = self
                    .module
                    .get_function(name)
                    .ok_or_else(|| anyhow!("Undefined function: {}", name))?;

                let mut llvm_args = Vec::new();
                for arg in args {
                    llvm_args.push(self.gen_operand(arg, body)?.into());
                }

                let call = self.builder.build_call(function, &llvm_args, "call_tmp")?;
                if let Some(val) = call.try_as_basic_value().left() {
                    let ptr = self.locals[dest];
                    self.builder.build_store(ptr, val)?;
                }
            }
            Instruction::Phi(_local, _entries) => {
                // TODO: Implement Phi properly
            }
            Instruction::Assert(op, _msg) => {
                let cond = self.gen_operand(op, body)?.into_int_value();
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let ok_bb = self.context.append_basic_block(current_fn, "assert_ok");
                let fail_bb = self.context.append_basic_block(current_fn, "assert_fail");

                self.builder
                    .build_conditional_branch(cond, ok_bb, fail_bb)?;

                // Fail block: abort
                self.builder.position_at_end(fail_bb);
                self.builder.build_unreachable()?;

                // OK block: continue
                self.builder.position_at_end(ok_bb);
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_terminator(&mut self, term: &Terminator, body: &MirBody) -> Result<()> {
        match term {
            Terminator::Return(op) => {
                if let Some(o) = op {
                    let val = self.gen_operand(o, body)?;
                    self.builder.build_return(Some(&val))?;
                } else {
                    self.builder.build_return(None)?;
                }
            }
            Terminator::Goto(target) => {
                self.builder
                    .build_unconditional_branch(self.blocks[target])?;
            }
            Terminator::SwitchInt(op, targets, default) => {
                let val = self.gen_operand(op, body)?.into_int_value();
                let cases: Vec<(IntValue<'ctx>, LlvmBasicBlock<'ctx>)> = targets
                    .iter()
                    .map(|(k, v)| (val.get_type().const_int(*k as u64, false), self.blocks[v]))
                    .collect();
                self.builder
                    .build_switch(val, self.blocks[default], &cases)?;
            }
            Terminator::Abort => {
                self.builder.build_unreachable()?;
            }
        }
        Ok(())
    }

    fn gen_rvalue(&mut self, rvalue: &Rvalue, body: &MirBody) -> Result<BasicValueEnum<'ctx>> {
        match rvalue {
            Rvalue::Use(op) => self.gen_operand(op, body),
            Rvalue::BinaryOp(op, lhs, rhs) => {
                let l = self.gen_operand(lhs, body)?;
                let r = self.gen_operand(rhs, body)?;
                self.gen_bin_op(*op, l, r)
            }
            Rvalue::UnaryOp(op, inner) => {
                let val = self.gen_operand(inner, body)?;
                self.gen_un_op(*op, val)
            }
            Rvalue::Ref(local, _is_mut) => Ok(self.locals[local].as_basic_value_enum()),
        }
    }

    fn gen_operand(&mut self, op: &Operand, body: &MirBody) -> Result<BasicValueEnum<'ctx>> {
        match op {
            Operand::Copy(l) | Operand::Move(l) => {
                let ptr = self.locals[l];
                let ty = self.llvm_type(&body.locals[l.0].ty)?;
                Ok(self.builder.build_load(ty, ptr, "load_tmp")?)
            }
            Operand::Constant(c) => match c {
                Constant::Int(i) => Ok(self.context.i32_type().const_int(*i as u64, false).into()),
                Constant::Float(f) => Ok(self.context.f64_type().const_float(*f).into()),
                Constant::Bool(b) => {
                    Ok(self.context.bool_type().const_int(*b as u64, false).into())
                }
                Constant::Str(_) => Err(anyhow!("String constants not yet implemented in codegen")),
            },
        }
    }

    fn gen_bin_op(
        &mut self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let l = lhs.into_int_value();
        let r = rhs.into_int_value();
        match op {
            BinOp::Add => Ok(self.builder.build_int_add(l, r, "add_tmp")?.into()),
            BinOp::Sub => Ok(self.builder.build_int_sub(l, r, "sub_tmp")?.into()),
            BinOp::Mul => Ok(self.builder.build_int_mul(l, r, "mul_tmp")?.into()),
            BinOp::Div => Ok(self.builder.build_int_signed_div(l, r, "div_tmp")?.into()),
            BinOp::Eq => Ok(self
                .builder
                .build_int_compare(IntPredicate::EQ, l, r, "eq_tmp")?
                .into()),
            BinOp::Ne => Ok(self
                .builder
                .build_int_compare(IntPredicate::NE, l, r, "ne_tmp")?
                .into()),
            BinOp::Lt => Ok(self
                .builder
                .build_int_compare(IntPredicate::SLT, l, r, "lt_tmp")?
                .into()),
            BinOp::Le => Ok(self
                .builder
                .build_int_compare(IntPredicate::SLE, l, r, "le_tmp")?
                .into()),
            BinOp::Gt => Ok(self
                .builder
                .build_int_compare(IntPredicate::SGT, l, r, "gt_tmp")?
                .into()),
            BinOp::Ge => Ok(self
                .builder
                .build_int_compare(IntPredicate::SGE, l, r, "ge_tmp")?
                .into()),
        }
    }

    fn gen_un_op(&mut self, op: UnOp, val: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        let v = val.into_int_value();
        match op {
            UnOp::Not => Ok(self.builder.build_not(v, "not_tmp")?.into()),
            UnOp::Neg => Ok(self.builder.build_int_neg(v, "neg_tmp")?.into()),
        }
    }

    fn llvm_type(&self, ty: &Type) -> Result<inkwell::types::BasicTypeEnum<'ctx>> {
        match ty {
            Type::Prim(p) => match p {
                PrimType::I32 => Ok(self.context.i32_type().into()),
                PrimType::F64 => Ok(self.context.f64_type().into()),
                PrimType::Bool => Ok(self.context.bool_type().into()),
                _ => Ok(self.context.i32_type().into()),
            },
            _ => Ok(self.context.i32_type().into()),
        }
    }
}

impl<'ctx, 'a> Codegen<'ctx, 'a> {
    pub fn new(context: &'ctx Context, name: &str, source: &'a str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Self {
            context,
            module,
            builder,
            source,
            variables: HashMap::new(),
        }
    }

    pub fn gen_module(&mut self, module: &ast::Module) -> Result<()> {
        for item in &module.items {
            self.gen_item(item)?;
        }
        Ok(())
    }

    fn gen_item(&mut self, item: &ast::Item) -> Result<()> {
        match item {
            ast::Item::Forge(f) => {
                self.gen_forge(f)?;
            }
            ast::Item::Impl(i) => {
                for it in &i.items {
                    self.gen_item(it)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_forge(&mut self, f: &ast::Forge) -> Result<FunctionValue<'ctx>> {
        // Check for intrinsic attribute
        let mut intrinsic_name = None;
        for attr in &f.attributes {
            if attr.name == "intrinsic" {
                if let Some(ast::Expr::Literal(ast::Literal::Str(name))) = attr.args.first() {
                    intrinsic_name = Some(name.clone());
                }
            }
        }

        let i32_type = self.context.i32_type();
        let f64_type = self.context.f64_type();
        let bool_type = self.context.bool_type();

        let ret_type = match &f.ret_type {
            ast::Type::Prim(p) if p == "i32" => i32_type.as_basic_type_enum(),
            ast::Type::Prim(p) if p == "f64" => f64_type.as_basic_type_enum(),
            ast::Type::Prim(p) if p == "bool" => bool_type.as_basic_type_enum(),
            _ => i32_type.as_basic_type_enum(), // Default for now
        };

        let mut arg_types = Vec::new();
        for p in &f.params {
            let ty = match &p.ty {
                ast::Type::Prim(pt) if pt == "i32" => i32_type.as_basic_type_enum(),
                ast::Type::Prim(pt) if pt == "f64" => f64_type.as_basic_type_enum(),
                ast::Type::Prim(pt) if pt == "bool" => bool_type.as_basic_type_enum(),
                _ => i32_type.as_basic_type_enum(),
            };
            arg_types.push(ty.into());
        }

        let fn_type = ret_type.fn_type(&arg_types, false);
        let function = self.module.add_function(&f.name, fn_type, None);

        if let Some(intrinsic) = intrinsic_name {
            self.gen_intrinsic_body(function, &intrinsic, &f.params)?;
        } else if let Some(_body) = &f.body {
            let basic_block = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(basic_block);

            // Minimal body logic for now
            self.builder.build_return(Some(&ret_type.const_zero()))?;
        }

        Ok(function)
    }

    fn gen_intrinsic_body(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        _params: &[ast::Param],
    ) -> Result<()> {
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        match name {
            "i32_abs" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                // LLVM doesn't have a direct 'abs' int instruction, we use the intrinsic
                let abs_intrinsic = self.get_intrinsic("llvm.abs.i32")?;
                let call = self.builder.build_call(
                    abs_intrinsic,
                    &[
                        val.into(),
                        self.context.bool_type().const_int(0, false).into(),
                    ],
                    "abs",
                )?;
                self.builder
                    .build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_sqrt" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let sqrt_intrinsic = self.get_intrinsic("llvm.sqrt.f64")?;
                let call = self
                    .builder
                    .build_call(sqrt_intrinsic, &[val.into()], "sqrt")?;
                self.builder
                    .build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_ceil" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let ceil_intrinsic = self.get_intrinsic("llvm.ceil.f64")?;
                let call = self
                    .builder
                    .build_call(ceil_intrinsic, &[val.into()], "ceil")?;
                self.builder
                    .build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_floor" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let floor_intrinsic = self.get_intrinsic("llvm.floor.f64")?;
                let call = self
                    .builder
                    .build_call(floor_intrinsic, &[val.into()], "floor")?;
                self.builder
                    .build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "bool_not" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                let res = self.builder.build_not(val, "not")?;
                self.builder.build_return(Some(&res))?;
            }
            _ => {
                return Err(anyhow!("Unknown intrinsic: {}", name));
            }
        }

        Ok(())
    }

    fn get_intrinsic(&self, name: &str) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function(name) {
            return Ok(f);
        }

        let i32_type = self.context.i32_type();
        let f64_type = self.context.f64_type();
        let bool_type = self.context.bool_type();

        match name {
            "llvm.abs.i32" => {
                let fn_type = i32_type.fn_type(&[i32_type.into(), bool_type.into()], false);
                Ok(self.module.add_function(name, fn_type, None))
            }
            "llvm.sqrt.f64" => {
                let fn_type = f64_type.fn_type(&[f64_type.into()], false);
                Ok(self.module.add_function(name, fn_type, None))
            }
            "llvm.ceil.f64" => {
                let fn_type = f64_type.fn_type(&[f64_type.into()], false);
                Ok(self.module.add_function(name, fn_type, None))
            }
            "llvm.floor.f64" => {
                let fn_type = f64_type.fn_type(&[f64_type.into()], false);
                Ok(self.module.add_function(name, fn_type, None))
            }
            _ => Err(anyhow!("Unsupported LLVM intrinsic: {}", name)),
        }
    }

    pub fn emit_llvm_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    pub fn run_jit(&self) -> Result<i32> {
        let execution_engine = self
            .module
            .create_jit_execution_engine(inkwell::OptimizationLevel::None)
            .map_err(|e| anyhow!("Failed to create JIT: {:?}", e))?;

        unsafe {
            let main_fn = execution_engine
                .get_function::<unsafe extern "C" fn() -> i32>("main")
                .map_err(|e| anyhow!("Failed to find main function in JIT: {:?}", e))?;

            println!("--- JIT Execution ---");
            let result = main_fn.call();
            println!("JIT Exit Code: {}", result);
            println!("----------------------\n");

            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_parser::ast;
    use izel_span::Span;

    #[test]
    fn test_intrinsic_codegen() -> Result<()> {
        let context = Context::create();
        let source = "";
        let mut codegen = Codegen::new(&context, "test", source);

        // forge abs(val: i32) -> i32
        let abs_forge = ast::Forge {
            name: "abs".to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            params: vec![ast::Param {
                name: "val".to_string(),
                ty: ast::Type::Prim("i32".to_string()),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("i32".to_string()),
            effects: vec![],
            is_flow: false,
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str("i32_abs".to_string()))],
                span: Span::dummy(),
            }],
            requires: vec![],
            ensures: vec![],
            body: None,
            span: Span::dummy(),
        };

        codegen.gen_item(&ast::Item::Forge(abs_forge))?;

        let ir = codegen.emit_llvm_ir();
        // println!("Generated IR:\n{}", ir);
        assert!(ir.contains("declare i32 @llvm.abs.i32(i32, i1")); // flexible match
        assert!(ir.contains("define i32 @abs(i32 %0)"));
        assert!(ir.contains("call i32 @llvm.abs.i32(i32 %0, i1 false)"));

        // Test bool_not
        let not_forge = ast::Forge {
            name: "not".to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            params: vec![ast::Param {
                name: "b".to_string(),
                ty: ast::Type::Prim("bool".to_string()),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("bool".to_string()),
            effects: vec![],
            is_flow: false,
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str(
                    "bool_not".to_string(),
                ))],
                span: Span::dummy(),
            }],
            requires: vec![],
            ensures: vec![],
            body: None,
            span: Span::dummy(),
        };

        codegen.gen_item(&ast::Item::Forge(not_forge))?;
        let ir2 = codegen.emit_llvm_ir();
        assert!(ir2.contains("xor i1 %0, true"));

        // Test f64_sqrt
        let sqrt_forge = ast::Forge {
            name: "sqrt".to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            params: vec![ast::Param {
                name: "v".to_string(),
                ty: ast::Type::Prim("f64".to_string()),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("f64".to_string()),
            effects: vec![],
            is_flow: false,
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str(
                    "f64_sqrt".to_string(),
                ))],
                span: Span::dummy(),
            }],
            requires: vec![],
            ensures: vec![],
            body: None,
            span: Span::dummy(),
        };

        codegen.gen_item(&ast::Item::Forge(sqrt_forge))?;
        let ir3 = codegen.emit_llvm_ir();
        assert!(ir3.contains("call double @llvm.sqrt.f64(double %0)"));

        Ok(())
    }

    #[test]
    fn test_mir_codegen() -> Result<()> {
        use izel_mir::LocalData;
        let context = Context::create();
        let module = context.create_module("test_mir");
        let builder = context.create_builder();

        // Target: forge add(a: i32, b: i32) -> i32 { return a + b }
        let i32_type = context.i32_type();
        let fn_type = i32_type.fn_type(&[i32_type.into(), i32_type.into()], false);
        let function = module.add_function("add", fn_type, None);

        let mut body = MirBody::new();
        body.locals = vec![
            LocalData {
                name: "ret".into(),
                ty: Type::Prim(PrimType::I32),
            }, // Local(0)
            LocalData {
                name: "a".into(),
                ty: Type::Prim(PrimType::I32),
            }, // Local(1)
            LocalData {
                name: "b".into(),
                ty: Type::Prim(PrimType::I32),
            }, // Local(2)
        ];

        let entry = body.entry;
        {
            let bb = body.blocks.node_weight_mut(entry).unwrap();
            // Local(0) = 10 + 20
            bb.instructions.push(Instruction::Assign(
                Local(0),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Constant(Constant::Int(10)),
                    Operand::Constant(Constant::Int(20)),
                ),
            ));
            bb.terminator = Some(Terminator::Return(Some(Operand::Copy(Local(0)))));
        }

        let mut mir_codegen = MirCodegen::new(&context, &module, &builder);
        mir_codegen.gen_mir_body(function, &body)?;

        let ir = module.print_to_string().to_string();
        assert!(ir.contains("define i32 @add(i32 %0, i32 %1)"));
        assert!(ir.contains("store i32 30, ptr %ret"));
        assert!(ir.contains("ret i32 %load_tmp"));

        Ok(())
    }
}
