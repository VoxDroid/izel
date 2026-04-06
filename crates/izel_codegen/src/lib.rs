//! LLVM code generation for Izel.

use anyhow::{anyhow, Result};
use inkwell::basic_block::BasicBlock as LlvmBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;

use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use izel_hir::{HirForge, HirItem};
use izel_mir::{
    BinOp, BlockId, Constant, Instruction, Local, MirBody, Operand, Rvalue, Terminator, UnOp,
};
use izel_parser::ast;
use izel_resolve::DefId;
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
            let ty = llvm_type_static(self.context, &local_data.ty)?;
            let ptr = self.builder.build_alloca(ty, &local_data.name)?;
            self.locals.insert(Local(i), ptr);
        }

        // 2.1 Store function parameters into locals (first N locals)
        for (i, llvm_param) in function.get_param_iter().enumerate() {
            let ptr = *self
                .locals
                .get(&Local(i))
                .ok_or_else(|| anyhow!("missing local slot for parameter {}", i))?;
            self.builder.build_store(ptr, llvm_param)?;
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
                let gen_name = format!("_iz_{}", name);
                let function = self
                    .module
                    .get_function(&gen_name)
                    .ok_or_else(|| anyhow!("Function not found: {}", gen_name))?;

                let mut llvm_args = Vec::new();
                for arg in args {
                    llvm_args.push(self.gen_operand(arg, body)?.into());
                }

                let call = self.builder.build_call(function, &llvm_args, "call_tmp")?;
                if let Some((dest_local, val)) = dest.as_ref().and_then(|dest_local| {
                    call.try_as_basic_value()
                        .left()
                        .map(|val| (*dest_local, val))
                }) {
                    let ptr = self.locals[&dest_local];
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
            Rvalue::Binary(op, lhs, rhs) => {
                let l = self.gen_operand(lhs, body)?;
                let r = self.gen_operand(rhs, body)?;
                self.gen_bin_op(*op, l, r)
            }
            Rvalue::Unary(op, inner) => {
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
                let ty = llvm_type_static(self.context, &body.locals[l.0].ty)?;
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
}

pub fn llvm_type_static<'ctx>(
    context: &'ctx Context,
    ty: &Type,
) -> Result<inkwell::types::BasicTypeEnum<'ctx>> {
    match ty {
        Type::Prim(p) => match p {
            PrimType::I8 | PrimType::U8 => Ok(context.i8_type().into()),
            PrimType::I16 | PrimType::U16 => Ok(context.i16_type().into()),
            PrimType::I32 | PrimType::U32 => Ok(context.i32_type().into()),
            PrimType::I64 | PrimType::U64 => Ok(context.i64_type().into()),
            PrimType::I128 | PrimType::U128 => Ok(context.i128_type().into()),
            PrimType::F32 => Ok(context.f32_type().into()),
            PrimType::F64 => Ok(context.f64_type().into()),
            PrimType::Bool => Ok(context.bool_type().into()),
            PrimType::Void => Err(anyhow!("Cannot represent void as basic type")),
            _ => Ok(context.i32_type().into()),
        },
        Type::Pointer(_inner, _, _) => Ok(context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0))
            .into()),
        _ => Ok(context.i32_type().into()),
    }
}

impl<'ctx, 'a> Codegen<'ctx, 'a> {
    fn llvm_type(&self, ty: &Type) -> Result<inkwell::types::BasicTypeEnum<'ctx>> {
        llvm_type_static(self.context, ty)
    }

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

    pub fn gen_module(
        &mut self,
        module: &izel_hir::HirModule,
        mir_bodies: &HashMap<DefId, MirBody>,
    ) -> Result<()> {
        // First pass: Declare all functions
        for item in &module.items {
            self.declare_item(item)?;
        }

        // Second pass: Generate all bodies
        for item in &module.items {
            self.gen_item(item, mir_bodies)?;
        }
        Ok(())
    }

    fn declare_item(&mut self, item: &HirItem) -> Result<()> {
        match item {
            HirItem::Forge(f) => {
                let mut arg_types = Vec::new();
                for p in &f.params {
                    arg_types.push(self.llvm_type(&p.ty)?.into());
                }

                let fn_type = if matches!(f.ret_type, Type::Prim(PrimType::Void)) {
                    self.context.void_type().fn_type(&arg_types, false)
                } else {
                    self.llvm_type(&f.ret_type)?.fn_type(&arg_types, false)
                };

                let gen_name = format!("_iz_{}", f.name);
                self.module.add_function(&gen_name, fn_type, None);
            }
            HirItem::Ward(w) => {
                for it in &w.items {
                    self.declare_item(it)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_item(&mut self, item: &HirItem, mir_bodies: &HashMap<DefId, MirBody>) -> Result<()> {
        match item {
            HirItem::Forge(f) => {
                self.gen_forge(f, mir_bodies)?;
            }
            HirItem::Ward(w) => {
                for it in &w.items {
                    self.gen_item(it, mir_bodies)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn gen_forge(
        &mut self,
        f: &HirForge,
        mir_bodies: &HashMap<DefId, MirBody>,
    ) -> Result<FunctionValue<'ctx>> {
        let gen_name = format!("_iz_{}", f.name);

        let function = self
            .module
            .get_function(&gen_name)
            .ok_or_else(|| anyhow!("Function not pre-declared: {}", gen_name))?;

        // Check for @intrinsic
        for attr in &f.attributes {
            if attr.name != "intrinsic" {
                continue;
            }
            if let Some(ast::Expr::Literal(ast::Literal::Str(name))) = attr.args.first() {
                let intrinsic_name = name.trim_matches('"');
                self.gen_intrinsic_body(function, intrinsic_name, &f.params)?;
                return Ok(function);
            }
        }

        if let Some(body) = mir_bodies.get(&f.def_id) {
            let mut mir_codegen = MirCodegen::new(self.context, &self.module, &self.builder);
            mir_codegen.gen_mir_body(function, body)?;
        } else if f.body.is_some() {
            // If we have a body but no MIR, it might be an issue or just not lowered yet
            // For now, we only support MIR-driven bodies for real execution
        }

        Ok(function)
    }

    fn gen_intrinsic_body(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        _params: &[izel_hir::HirParam],
    ) -> Result<()> {
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        match name {
            "i32_abs" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                // LLVM doesn't have a direct 'abs' int instruction, we use the intrinsic
                let abs_intrinsic = self.get_intrinsic("llvm.abs.i32")?;
                let abs_args = [
                    val.into(),
                    self.context.bool_type().const_int(0, false).into(),
                ];
                let call = self.builder.build_call(abs_intrinsic, &abs_args, "abs")?;
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
            "io_print_int" => {
                let val = function.get_nth_param(0).unwrap();
                let printf = self.get_printf()?;
                let format_str = self.builder.build_global_string_ptr("%d\n", "format_int")?;
                let printf_args = [format_str.as_pointer_value().into(), val.into()];
                self.builder.build_call(printf, &printf_args, "printf")?;
                self.builder.build_return(None)?;
            }
            "io_print_newline" => {
                let printf = self.get_printf()?;
                let format_str = self.builder.build_global_string_ptr("\n", "format_nl")?;
                let printf_args = [format_str.as_pointer_value().into()];
                self.builder.build_call(printf, &printf_args, "printf")?;
                self.builder.build_return(None)?;
            }
            "mem_alloc" => {
                let size = function.get_nth_param(0).unwrap().into_int_value();
                let malloc = self.get_malloc()?;
                let call = self.builder.build_call(malloc, &[size.into()], "malloc")?;
                self.builder
                    .build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "mem_free" => {
                let ptr = function.get_nth_param(0).unwrap().into_pointer_value();
                let free = self.get_free()?;
                self.builder.build_call(free, &[ptr.into()], "free")?;
                self.builder.build_return(None)?;
            }
            "bool_not" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                let res = self.builder.build_not(val, "not")?;
                self.builder.build_return(Some(&res))?;
            }
            "simd_i32x4_sum" => {
                let v4i32 = self.context.i32_type().vec_type(4);

                let mut vec = v4i32.get_undef();
                for i in 0..4u32 {
                    let lane = function.get_nth_param(i).unwrap().into_int_value();
                    let lane_index = self.context.i32_type().const_int(i as u64, false);
                    vec = self
                        .builder
                        .build_insert_element(vec, lane, lane_index, "ins")?;
                }

                let reduce_add = self.get_intrinsic("llvm.vector.reduce.add.v4i32")?;
                let call = self
                    .builder
                    .build_call(reduce_add, &[vec.into()], "simd_reduce_add")?;
                let sum = call.try_as_basic_value().left().unwrap().into_int_value();
                self.builder.build_return(Some(&sum))?;
            }
            _ => {
                return Err(anyhow!("Unknown intrinsic: {}", name));
            }
        }

        Ok(())
    }

    fn get_printf(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("printf") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = i32_type.fn_type(&[ptr_type.into()], true);
        Ok(self.module.add_function("printf", fn_type, None))
    }

    fn get_malloc(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("malloc") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let size_type = self.context.i64_type(); // Assuming 64-bit size_t for now
        let fn_type = ptr_type.fn_type(&[size_type.into()], false);
        Ok(self.module.add_function("malloc", fn_type, None))
    }

    fn get_free(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("free") {
            return Ok(f);
        }
        let void_type = self.context.void_type();
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = void_type.fn_type(&[ptr_type.into()], false);
        Ok(self.module.add_function("free", fn_type, None))
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
            "llvm.vector.reduce.add.v4i32" => {
                let v4i32_type = i32_type.vec_type(4);
                let fn_type = i32_type.fn_type(&[v4i32_type.into()], false);
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
                .get_function::<unsafe extern "C" fn() -> i32>("_iz_main")
                .map_err(|e| anyhow!("LLVM JIT Error: {}", e))?;

            println!("--- JIT Execution ---");
            let res = main_fn.call();
            println!("JIT Exit Code: {}", res);
            println!("----------------------");
            Ok(res)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_hir::*;
    use izel_parser::ast;
    use izel_span::Span;
    use izel_typeck::type_system::{Lifetime, PrimType, Type};

    fn param(id: usize, name: &str, ty: Type) -> HirParam {
        HirParam {
            name: name.to_string(),
            def_id: DefId(id),
            ty,
            default_value: None,
            is_variadic: false,
            span: Span::dummy(),
        }
    }

    fn intrinsic_forge(
        id: usize,
        name: &str,
        intrinsic: &str,
        params: Vec<HirParam>,
        ret_type: Type,
    ) -> HirForge {
        HirForge {
            name: name.to_string(),
            name_span: Span::dummy(),
            def_id: DefId(id),
            params,
            ret_type,
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str(intrinsic.to_string()))],
                span: Span::dummy(),
            }],
            body: None,
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_intrinsic_codegen() -> Result<()> {
        let context = Context::create();
        let source = "";
        let mut codegen = Codegen::new(&context, "test", source);

        // forge abs(val: i32) -> i32
        let abs_forge = HirForge {
            name: "abs".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(1),
            params: vec![HirParam {
                name: "val".to_string(),
                def_id: DefId(2),
                ty: Type::Prim(PrimType::I32),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: Type::Prim(PrimType::I32),
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str("i32_abs".to_string()))],
                span: Span::dummy(),
            }],
            body: None,
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };

        let mir_bodies = HashMap::new();
        let item = HirItem::Forge(Box::new(abs_forge));
        codegen.declare_item(&item)?;
        codegen.gen_item(&item, &mir_bodies)?;

        let ir = codegen.emit_llvm_ir();
        // println!("Generated IR:\n{}", ir);
        assert!(ir.contains("declare i32 @llvm.abs.i32(i32, i1")); // flexible match
        assert!(ir.contains("define i32 @_iz_abs(i32 %0)"));
        assert!(ir.contains("call i32 @llvm.abs.i32(i32 %0, i1 false)"));

        // Test bool_not
        let not_forge = HirForge {
            name: "not".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(3),
            params: vec![HirParam {
                name: "b".to_string(),
                def_id: DefId(4),
                ty: Type::Prim(PrimType::Bool),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: Type::Prim(PrimType::Bool),
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str(
                    "bool_not".to_string(),
                ))],
                span: Span::dummy(),
            }],
            body: Some(HirBlock {
                stmts: vec![HirStmt::Let {
                    name: "x".to_string(),
                    def_id: DefId(5),
                    ty: Type::Prim(PrimType::I32),
                    init: Some(HirExpr::Literal(ast::Literal::Int(30))),
                    span: Span::dummy(),
                }],
                expr: None,
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };

        let item2 = HirItem::Forge(Box::new(not_forge));
        codegen.declare_item(&item2)?;
        codegen.gen_item(&item2, &mir_bodies)?;
        let ir2 = codegen.emit_llvm_ir();
        assert!(ir2.contains("xor i1 %0, true"));

        // Test f64_sqrt
        let sqrt_forge = HirForge {
            name: "sqrt".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(6),
            params: vec![HirParam {
                name: "v".to_string(),
                def_id: DefId(7),
                ty: Type::Prim(PrimType::F64),
                default_value: None,
                is_variadic: false,
                span: Span::dummy(),
            }],
            ret_type: Type::Prim(PrimType::F64),
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

        let item3 = HirItem::Forge(Box::new(sqrt_forge));
        codegen.declare_item(&item3)?;
        codegen.gen_item(&item3, &mir_bodies)?;
        let ir3 = codegen.emit_llvm_ir();
        assert!(ir3.contains("call double @llvm.sqrt.f64(double %0)"));

        // Test simd_i32x4_sum
        let simd_sum_forge = HirForge {
            name: "simd_sum".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(8),
            params: vec![
                HirParam {
                    name: "a".to_string(),
                    def_id: DefId(9),
                    ty: Type::Prim(PrimType::I32),
                    default_value: None,
                    is_variadic: false,
                    span: Span::dummy(),
                },
                HirParam {
                    name: "b".to_string(),
                    def_id: DefId(10),
                    ty: Type::Prim(PrimType::I32),
                    default_value: None,
                    is_variadic: false,
                    span: Span::dummy(),
                },
                HirParam {
                    name: "c".to_string(),
                    def_id: DefId(11),
                    ty: Type::Prim(PrimType::I32),
                    default_value: None,
                    is_variadic: false,
                    span: Span::dummy(),
                },
                HirParam {
                    name: "d".to_string(),
                    def_id: DefId(12),
                    ty: Type::Prim(PrimType::I32),
                    default_value: None,
                    is_variadic: false,
                    span: Span::dummy(),
                },
            ],
            ret_type: Type::Prim(PrimType::I32),
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str(
                    "simd_i32x4_sum".to_string(),
                ))],
                span: Span::dummy(),
            }],
            requires: vec![],
            ensures: vec![],
            body: None,
            span: Span::dummy(),
        };

        let item4 = HirItem::Forge(Box::new(simd_sum_forge));
        codegen.declare_item(&item4)?;
        codegen.gen_item(&item4, &mir_bodies)?;
        let ir4 = codegen.emit_llvm_ir();
        assert!(ir4.contains("declare i32 @llvm.vector.reduce.add.v4i32(<4 x i32>)"));
        assert!(ir4.contains("define i32 @_iz_simd_sum(i32 %0, i32 %1, i32 %2, i32 %3)"));
        assert!(ir4.contains("call i32 @llvm.vector.reduce.add.v4i32(<4 x i32>"));

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
                Rvalue::Binary(
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

    #[test]
    fn test_additional_intrinsics_and_helper_caches() -> Result<()> {
        let context = Context::create();
        let mut codegen = Codegen::new(&context, "extra_intrinsics", "");
        let mir_bodies = HashMap::new();

        let items = vec![
            HirItem::Forge(Box::new(intrinsic_forge(
                100,
                "print_int",
                "io_print_int",
                vec![param(101, "v", Type::Prim(PrimType::I32))],
                Type::Prim(PrimType::Void),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                102,
                "print_nl",
                "io_print_newline",
                vec![],
                Type::Prim(PrimType::Void),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                103,
                "alloc",
                "mem_alloc",
                vec![param(104, "size", Type::Prim(PrimType::I64))],
                Type::Pointer(Box::new(Type::Prim(PrimType::I8)), false, Lifetime::Static),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                105,
                "free_mem",
                "mem_free",
                vec![param(
                    106,
                    "ptr",
                    Type::Pointer(Box::new(Type::Prim(PrimType::I8)), false, Lifetime::Static),
                )],
                Type::Prim(PrimType::Void),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                107,
                "ceil",
                "f64_ceil",
                vec![param(108, "x", Type::Prim(PrimType::F64))],
                Type::Prim(PrimType::F64),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                109,
                "floor",
                "f64_floor",
                vec![param(110, "x", Type::Prim(PrimType::F64))],
                Type::Prim(PrimType::F64),
            ))),
        ];

        for item in &items {
            codegen.declare_item(item)?;
            codegen.gen_item(item, &mir_bodies)?;
        }

        let ir = codegen.emit_llvm_ir();
        assert!(ir.contains("define void @_iz_print_int(i32 %0)"));
        assert!(ir.contains("define void @_iz_print_nl()"));
        assert!(ir.contains("declare i32 @printf(ptr, ...)"));
        assert!(ir.contains("declare ptr @malloc(i64)"));
        assert!(ir.contains("declare void @free(ptr)"));
        assert!(ir.contains("call double @llvm.ceil.f64(double %0)"));
        assert!(ir.contains("call double @llvm.floor.f64(double %0)"));

        // Repeated lookups should return existing declarations.
        let printf1 = codegen.get_printf()?;
        let printf2 = codegen.get_printf()?;
        assert_eq!(
            printf1.as_global_value().as_pointer_value(),
            printf2.as_global_value().as_pointer_value()
        );

        let malloc1 = codegen.get_malloc()?;
        let malloc2 = codegen.get_malloc()?;
        assert_eq!(
            malloc1.as_global_value().as_pointer_value(),
            malloc2.as_global_value().as_pointer_value()
        );

        let free1 = codegen.get_free()?;
        let free2 = codegen.get_free()?;
        assert_eq!(
            free1.as_global_value().as_pointer_value(),
            free2.as_global_value().as_pointer_value()
        );

        let err = codegen
            .get_intrinsic("llvm.unknown.thing")
            .expect_err("unknown llvm intrinsic should error");
        assert!(err.to_string().contains("Unsupported LLVM intrinsic"));

        Ok(())
    }

    #[test]
    fn test_codegen_error_paths_for_intrinsics_and_predeclare() {
        let context = Context::create();
        let mut codegen = Codegen::new(&context, "errors", "");
        let mir_bodies = HashMap::new();

        let unknown = HirItem::Forge(Box::new(intrinsic_forge(
            120,
            "mystery",
            "totally_unknown",
            vec![],
            Type::Prim(PrimType::Void),
        )));
        codegen
            .declare_item(&unknown)
            .expect("declare unknown intrinsic forge");
        let err = codegen
            .gen_item(&unknown, &mir_bodies)
            .expect_err("unknown intrinsic should fail during body generation");
        assert!(err.to_string().contains("Unknown intrinsic"));

        let not_declared = HirItem::Forge(Box::new(HirForge {
            name: "plain".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(121),
            params: vec![],
            ret_type: Type::Prim(PrimType::I32),
            attributes: vec![],
            body: None,
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        }));
        let err = codegen
            .gen_item(&not_declared, &mir_bodies)
            .expect_err("non-declared forge should fail");
        assert!(err.to_string().contains("not pre-declared"));
    }

    #[test]
    fn test_mir_codegen_control_flow_and_operand_variants() -> Result<()> {
        use izel_mir::LocalData;

        let context = Context::create();
        let module = context.create_module("test_mir_flow");
        let builder = context.create_builder();

        let i32_ty = context.i32_type();
        let bool_ty = context.bool_type();
        let void_ty = context.void_type();
        module.add_function(
            "_iz_callee",
            i32_ty.fn_type(&[i32_ty.into(), i32_ty.into()], false),
            None,
        );
        module.add_function("_iz_sink", void_ty.fn_type(&[bool_ty.into()], false), None);

        let function = module.add_function("flow", i32_ty.fn_type(&[], false), None);

        let mut body = MirBody::new();
        let branch_bb = body.blocks.add_node(izel_mir::BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });
        let exit_bb = body.blocks.add_node(izel_mir::BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });

        body.locals = vec![
            LocalData {
                name: "ret".into(),
                ty: Type::Prim(PrimType::I32),
            },
            LocalData {
                name: "tmp".into(),
                ty: Type::Prim(PrimType::I32),
            },
            LocalData {
                name: "flag".into(),
                ty: Type::Prim(PrimType::Bool),
            },
            LocalData {
                name: "flt".into(),
                ty: Type::Prim(PrimType::F64),
            },
            LocalData {
                name: "ptr".into(),
                ty: Type::Pointer(Box::new(Type::Prim(PrimType::I32)), false, Lifetime::Static),
            },
        ];

        {
            let entry = body.entry;
            let bb = body.blocks.node_weight_mut(entry).expect("entry block");
            bb.instructions.push(Instruction::Assign(
                Local(0),
                Rvalue::Use(Operand::Constant(Constant::Int(5))),
            ));
            bb.instructions.push(Instruction::Assign(
                Local(1),
                Rvalue::Unary(UnOp::Neg, Operand::Constant(Constant::Int(2))),
            ));
            bb.instructions.push(Instruction::Assign(
                Local(2),
                Rvalue::Use(Operand::Constant(Constant::Bool(true))),
            ));
            bb.instructions.push(Instruction::Assign(
                Local(3),
                Rvalue::Use(Operand::Constant(Constant::Float(3.5))),
            ));
            bb.instructions
                .push(Instruction::Assign(Local(4), Rvalue::Ref(Local(1), false)));
            bb.instructions.push(Instruction::Call(
                Some(Local(0)),
                "callee".to_string(),
                vec![Operand::Copy(Local(0)), Operand::Move(Local(1))],
            ));
            bb.instructions.push(Instruction::Call(
                None,
                "sink".to_string(),
                vec![Operand::Copy(Local(2))],
            ));
            bb.instructions.push(Instruction::Assert(
                Operand::Copy(Local(2)),
                "must hold".to_string(),
            ));
            bb.instructions.push(Instruction::Phi(Local(0), vec![]));
            bb.instructions.push(Instruction::StorageLive(Local(0)));
            bb.instructions
                .push(Instruction::ZoneEnter("z".to_string()));
            bb.terminator = Some(Terminator::SwitchInt(
                Operand::Copy(Local(0)),
                vec![(5, branch_bb)],
                exit_bb,
            ));
        }

        {
            let bb = body
                .blocks
                .node_weight_mut(branch_bb)
                .expect("branch block");
            bb.instructions.push(Instruction::Assign(
                Local(0),
                Rvalue::Binary(
                    BinOp::Sub,
                    Operand::Copy(Local(0)),
                    Operand::Constant(Constant::Int(1)),
                ),
            ));
            bb.terminator = Some(Terminator::Goto(exit_bb));
        }

        {
            let bb = body.blocks.node_weight_mut(exit_bb).expect("exit block");
            bb.instructions.push(Instruction::ZoneExit("z".to_string()));
            bb.terminator = Some(Terminator::Return(Some(Operand::Copy(Local(0)))));
        }

        let mut mir_codegen = MirCodegen::new(&context, &module, &builder);
        mir_codegen.gen_mir_body(function, &body)?;

        let str_err = mir_codegen
            .gen_operand(&Operand::Constant(Constant::Str("x".to_string())), &body)
            .expect_err("string constants are not yet supported in codegen");
        assert!(str_err
            .to_string()
            .contains("String constants not yet implemented"));

        let ir = module.print_to_string().to_string();
        assert!(ir.contains("call i32 @_iz_callee(i32"));
        assert!(ir.contains("call void @_iz_sink(i1"));
        assert!(ir.contains("switch i32"));
        assert!(ir.contains("assert_fail"));
        assert!(ir.contains("br label"));
        assert!(ir.contains("ret i32"));

        Ok(())
    }

    #[test]
    fn test_mir_codegen_abort_return_none_and_missing_terminator() -> Result<()> {
        let context = Context::create();
        let module = context.create_module("test_mir_misc_terms");
        let builder = context.create_builder();

        let void_fn_ty = context.void_type().fn_type(&[], false);

        let abort_fn = module.add_function("abort_fn", void_fn_ty, None);
        let mut abort_body = MirBody::new();
        abort_body.blocks[abort_body.entry].terminator = Some(Terminator::Abort);
        MirCodegen::new(&context, &module, &builder).gen_mir_body(abort_fn, &abort_body)?;

        let ret_void_fn = module.add_function("ret_void_fn", void_fn_ty, None);
        let mut ret_void_body = MirBody::new();
        ret_void_body.blocks[ret_void_body.entry].terminator = Some(Terminator::Return(None));
        MirCodegen::new(&context, &module, &builder).gen_mir_body(ret_void_fn, &ret_void_body)?;

        let no_term_fn = module.add_function("no_term_fn", void_fn_ty, None);
        let no_term_body = MirBody::new();
        MirCodegen::new(&context, &module, &builder).gen_mir_body(no_term_fn, &no_term_body)?;

        let ir = module.print_to_string().to_string();
        assert!(ir.contains("define void @abort_fn()"));
        assert!(ir.contains("define void @ret_void_fn()"));
        assert!(ir.contains("ret void"));
        assert!(ir.contains("define void @no_term_fn()"));
        assert!(ir.contains("unreachable"));

        Ok(())
    }

    #[test]
    fn test_codegen_covers_remaining_bin_un_and_type_arms() -> Result<()> {
        let context = Context::create();
        let module = context.create_module("ops_and_types");
        let builder = context.create_builder();

        let fn_ty = context.i32_type().fn_type(&[], false);
        let function = module.add_function("ops", fn_ty, None);
        let entry = context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        let mut mir_codegen = MirCodegen::new(&context, &module, &builder);
        let lhs = context.i32_type().const_int(8, false).into();
        let rhs = context.i32_type().const_int(2, false).into();

        for op in [
            BinOp::Mul,
            BinOp::Div,
            BinOp::Eq,
            BinOp::Ne,
            BinOp::Lt,
            BinOp::Le,
            BinOp::Gt,
            BinOp::Ge,
        ] {
            let _ = mir_codegen.gen_bin_op(op, lhs, rhs)?;
        }

        let _ = mir_codegen.gen_un_op(UnOp::Not, context.bool_type().const_int(1, false).into())?;
        builder.build_return(Some(&context.i32_type().const_int(0, false)))?;

        let i8 = llvm_type_static(&context, &Type::Prim(PrimType::I8))?;
        assert!(matches!(
            i8,
            inkwell::types::BasicTypeEnum::IntType(t) if t.get_bit_width() == 8
        ));

        let i16 = llvm_type_static(&context, &Type::Prim(PrimType::I16))?;
        assert!(matches!(
            i16,
            inkwell::types::BasicTypeEnum::IntType(t) if t.get_bit_width() == 16
        ));

        let i128 = llvm_type_static(&context, &Type::Prim(PrimType::I128))?;
        assert!(matches!(
            i128,
            inkwell::types::BasicTypeEnum::IntType(t) if t.get_bit_width() == 128
        ));

        let f32 = llvm_type_static(&context, &Type::Prim(PrimType::F32))?;
        assert!(matches!(f32, inkwell::types::BasicTypeEnum::FloatType(_)));

        Ok(())
    }

    #[test]
    fn test_codegen_body_without_mir_and_intrinsic_cache_hit() -> Result<()> {
        let context = Context::create();
        let mut codegen = Codegen::new(&context, "body_without_mir", "");

        let item = HirItem::Forge(Box::new(HirForge {
            name: "no_mir_body".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(900),
            params: vec![],
            ret_type: Type::Prim(PrimType::I32),
            attributes: vec![],
            body: Some(HirBlock {
                stmts: vec![],
                expr: Some(Box::new(HirExpr::Literal(ast::Literal::Int(0)))),
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        }));

        codegen.declare_item(&item)?;
        codegen.gen_item(&item, &HashMap::new())?;

        let first = codegen.get_intrinsic("llvm.abs.i32")?;
        let second = codegen.get_intrinsic("llvm.abs.i32")?;
        assert_eq!(
            first.as_global_value().as_pointer_value(),
            second.as_global_value().as_pointer_value()
        );

        Ok(())
    }

    #[test]
    fn test_codegen_skips_non_intrinsic_and_non_string_intrinsic_attrs() -> Result<()> {
        let context = Context::create();
        let mut codegen = Codegen::new(&context, "attr_fallbacks", "");

        let item = HirItem::Forge(Box::new(HirForge {
            name: "attr_paths".to_string(),
            name_span: Span::dummy(),
            def_id: DefId(901),
            params: vec![],
            ret_type: Type::Prim(PrimType::I32),
            attributes: vec![
                ast::Attribute {
                    name: "inline".to_string(),
                    args: vec![],
                    span: Span::dummy(),
                },
                ast::Attribute {
                    name: "intrinsic".to_string(),
                    args: vec![ast::Expr::Literal(ast::Literal::Int(1))],
                    span: Span::dummy(),
                },
            ],
            body: None,
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        }));

        codegen.declare_item(&item)?;
        codegen.gen_item(&item, &HashMap::new())?;

        assert!(codegen.module.get_function("_iz_attr_paths").is_some());
        Ok(())
    }

    #[test]
    fn test_codegen_exercises_remaining_intrinsic_paths() -> Result<()> {
        let context = Context::create();
        let mut codegen = Codegen::new(&context, "intrinsic_paths", "");

        let i32_abs = HirItem::Forge(Box::new(intrinsic_forge(
            902,
            "abs_again",
            "i32_abs",
            vec![param(903, "v", Type::Prim(PrimType::I32))],
            Type::Prim(PrimType::I32),
        )));
        let io_print_int = HirItem::Forge(Box::new(intrinsic_forge(
            904,
            "print_int",
            "io_print_int",
            vec![param(905, "v", Type::Prim(PrimType::I32))],
            Type::Prim(PrimType::Void),
        )));
        let io_print_newline = HirItem::Forge(Box::new(intrinsic_forge(
            906,
            "print_newline",
            "io_print_newline",
            vec![],
            Type::Prim(PrimType::Void),
        )));
        let simd_sum = HirItem::Forge(Box::new(intrinsic_forge(
            907,
            "simd_sum",
            "simd_i32x4_sum",
            vec![
                param(908, "a", Type::Prim(PrimType::I32)),
                param(909, "b", Type::Prim(PrimType::I32)),
                param(910, "c", Type::Prim(PrimType::I32)),
                param(911, "d", Type::Prim(PrimType::I32)),
            ],
            Type::Prim(PrimType::I32),
        )));

        let items = [i32_abs, io_print_int, io_print_newline, simd_sum];
        for item in &items {
            codegen.declare_item(item)?;
            codegen.gen_item(item, &HashMap::new())?;
        }

        let ir = codegen.emit_llvm_ir();
        assert!(ir.contains("@llvm.abs.i32"));
        assert!(ir.contains("@printf"));
        assert!(ir.contains("@llvm.vector.reduce.add.v4i32"));
        Ok(())
    }
}
