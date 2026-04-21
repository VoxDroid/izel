//! LLVM code generation for Izel.

use anyhow::{anyhow, Result};
use inkwell::basic_block::BasicBlock as LlvmBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;

use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{FloatPredicate, IntPredicate};
use izel_hir::{HirForge, HirItem};
use izel_mir::{
    BinOp, BlockId, Constant, Instruction, Local, MirBody, Operand, Rvalue, Terminator, UnOp,
};
use izel_parser::ast;
use izel_resolve::DefId;
use izel_typeck::type_system::{PrimType, Type};
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicI32, Ordering};

static IO_LAST_STATUS: AtomicI32 = AtomicI32::new(0);

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn set_io_last_status(status: i32) {
    IO_LAST_STATUS.store(status, Ordering::Relaxed);
}

fn io_status_from_error(err: &std::io::Error) -> i32 {
    err.raw_os_error().unwrap_or(-1)
}

fn io_error_kind_from_status(status: i32) -> i32 {
    match status {
        0 => 0,
        -2 => 5,
        -12 => 7,
        2 | 3 => 1,
        5 | 13 => 2,
        17 | 183 => 3,
        4 => 4,
        22 | 87 => 6,
        _ => 255,
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(LUT[(byte >> 4) as usize] as char);
        out.push(LUT[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(text: &str) -> Result<Vec<u8>, i32> {
    let trimmed = text.trim();
    if !trimmed.len().is_multiple_of(2) {
        return Err(-2);
    }

    let bytes = trimmed.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = hex_nibble(bytes[idx]).ok_or(-2)?;
        let lo = hex_nibble(bytes[idx + 1]).ok_or(-2)?;
        out.push((hi << 4) | lo);
        idx += 2;
    }

    Ok(out)
}

fn c_ptr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let c_str = unsafe { CStr::from_ptr(ptr) };
    Some(c_str.to_string_lossy().into_owned())
}

fn alloc_runtime_string(value: &str) -> *mut c_char {
    let mut sanitized = Vec::with_capacity(value.len() + 1);
    sanitized.extend(value.as_bytes().iter().copied().filter(|b| *b != 0));
    sanitized.push(0);

    let out = unsafe { malloc(sanitized.len()) as *mut u8 };
    if out.is_null() {
        set_io_last_status(-12);
        return std::ptr::null_mut();
    }

    unsafe {
        std::ptr::copy_nonoverlapping(sanitized.as_ptr(), out, sanitized.len());
    }
    out.cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn izel_io_last_status() -> i32 {
    IO_LAST_STATUS.load(Ordering::Relaxed)
}

#[no_mangle]
pub extern "C" fn izel_io_last_error_kind() -> i32 {
    io_error_kind_from_status(IO_LAST_STATUS.load(Ordering::Relaxed))
}

#[no_mangle]
pub extern "C" fn izel_io_read_stdin() -> *mut c_char {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            set_io_last_status(0);
            alloc_runtime_string(&input)
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            alloc_runtime_string("")
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_read_file(path: *const c_char) -> *mut c_char {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return alloc_runtime_string("");
    };

    match std::fs::read_to_string(path) {
        Ok(contents) => {
            set_io_last_status(0);
            alloc_runtime_string(&contents)
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            alloc_runtime_string("")
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_write_file(path: *const c_char, content: *const c_char) -> i32 {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return -1;
    };
    let Some(content) = c_ptr_to_string(content) else {
        set_io_last_status(-1);
        return -1;
    };

    match std::fs::write(path, content.as_bytes()) {
        Ok(()) => {
            set_io_last_status(0);
            i32::try_from(content.len()).unwrap_or(i32::MAX)
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_append_file(path: *const c_char, content: *const c_char) -> i32 {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return -1;
    };
    let Some(content) = c_ptr_to_string(content) else {
        set_io_last_status(-1);
        return -1;
    };

    let open_result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path);

    match open_result {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(()) => {
                set_io_last_status(0);
                i32::try_from(content.len()).unwrap_or(i32::MAX)
            }
            Err(err) => {
                set_io_last_status(io_status_from_error(&err));
                -1
            }
        },
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_remove_file(path: *const c_char) -> i32 {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return -1;
    };

    match std::fs::remove_file(path) {
        Ok(()) => {
            set_io_last_status(0);
            0
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_file_exists(path: *const c_char) -> i32 {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return 0;
    };

    match std::fs::metadata(path) {
        Ok(_) => {
            set_io_last_status(0);
            1
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            set_io_last_status(0);
            0
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_list_dir(path: *const c_char) -> *mut c_char {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return alloc_runtime_string("");
    };

    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort_unstable();

            let mut out = String::new();
            for name in names {
                out.push_str(&name);
                out.push('\n');
            }

            set_io_last_status(0);
            alloc_runtime_string(&out)
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            alloc_runtime_string("")
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_list_dir_structured(path: *const c_char) -> *mut c_char {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return alloc_runtime_string("");
    };

    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut rows: Vec<(String, &'static str)> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let kind = match entry.file_type() {
                        Ok(file_type) if file_type.is_dir() => "dir",
                        Ok(file_type) if file_type.is_file() => "file",
                        Ok(file_type) if file_type.is_symlink() => "symlink",
                        Ok(_) => "other",
                        Err(_) => "other",
                    };
                    (name, kind)
                })
                .collect();
            rows.sort_unstable_by(|left, right| left.0.cmp(&right.0));

            let mut out = String::new();
            for (name, kind) in rows {
                out.push_str(&name);
                out.push('\t');
                out.push_str(kind);
                out.push('\n');
            }

            set_io_last_status(0);
            alloc_runtime_string(&out)
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            alloc_runtime_string("")
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_read_file_bytes_hex(path: *const c_char) -> *mut c_char {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return alloc_runtime_string("");
    };

    match std::fs::read(path) {
        Ok(contents) => {
            set_io_last_status(0);
            alloc_runtime_string(&encode_hex(&contents))
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            alloc_runtime_string("")
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_write_file_bytes_hex(
    path: *const c_char,
    content_hex: *const c_char,
) -> i32 {
    let Some(path) = c_ptr_to_string(path) else {
        set_io_last_status(-1);
        return -1;
    };
    let Some(content_hex) = c_ptr_to_string(content_hex) else {
        set_io_last_status(-1);
        return -1;
    };

    let bytes = match decode_hex(&content_hex) {
        Ok(bytes) => bytes,
        Err(status) => {
            set_io_last_status(status);
            return -1;
        }
    };
    let bytes_written = i32::try_from(bytes.len()).unwrap_or(i32::MAX);

    match std::fs::write(path, &bytes) {
        Ok(()) => {
            set_io_last_status(0);
            bytes_written
        }
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_read_stdin_int() -> i32 {
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => match input.trim().parse::<i32>() {
            Ok(value) => {
                set_io_last_status(0);
                value
            }
            Err(_) => {
                set_io_last_status(-2);
                0
            }
        },
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn izel_io_read_stdin_float() -> f64 {
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => match input.trim().parse::<f64>() {
            Ok(value) => {
                set_io_last_status(0);
                value
            }
            Err(_) => {
                set_io_last_status(-2);
                0.0
            }
        },
        Err(err) => {
            set_io_last_status(io_status_from_error(&err));
            0.0
        }
    }
}

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
                let mut llvm_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                for arg in args {
                    llvm_args.push(self.gen_operand(arg, body)?.into());
                }

                let function = if let Some(existing) = self.module.get_function(&gen_name) {
                    existing
                } else {
                    let fn_type = if let Some(dest_local) = dest {
                        let ret_ty = llvm_type_static(self.context, &body.locals[dest_local.0].ty)?;
                        ret_ty.fn_type(&[], true)
                    } else {
                        self.context.void_type().fn_type(&[], true)
                    };

                    self.module.add_function(&gen_name, fn_type, None)
                };

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
            Instruction::Phi(local, entries) => {
                if entries.is_empty() {
                    return Ok(());
                }

                let ty = llvm_type_static(self.context, &body.locals[local.0].ty)?;
                let phi = self.builder.build_phi(ty, "phi_tmp")?;

                let mut incoming_values = Vec::new();
                let temp_builder = self.context.create_builder();

                for (pred, src_local) in entries {
                    let pred_bb = self.blocks[pred];
                    if let Some(term) = pred_bb.get_terminator() {
                        temp_builder.position_before(&term);
                    } else {
                        temp_builder.position_at_end(pred_bb);
                    }

                    let src_ptr = self.locals[src_local];
                    let src_ty = llvm_type_static(self.context, &body.locals[src_local.0].ty)?;
                    let loaded = temp_builder.build_load(src_ty, src_ptr, "phi_in")?;
                    incoming_values.push((loaded, pred_bb));
                }

                let incoming_refs = incoming_values
                    .iter()
                    .map(|(val, bb)| (val as &dyn BasicValue<'ctx>, *bb))
                    .collect::<Vec<_>>();
                phi.add_incoming(&incoming_refs);

                let ptr = self.locals[local];
                self.builder.build_store(ptr, phi.as_basic_value())?;
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
                Constant::Str(s) => {
                    let literal = decode_izel_string_literal(s);
                    let global = self
                        .builder
                        .build_global_string_ptr(&literal, "str_const")?;
                    Ok(global.as_pointer_value().into())
                }
            },
        }
    }

    fn gen_bin_op(
        &mut self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        if lhs.is_float_value() || rhs.is_float_value() {
            let l = if lhs.is_float_value() {
                lhs.into_float_value()
            } else {
                self.builder.build_signed_int_to_float(
                    lhs.into_int_value(),
                    self.context.f64_type(),
                    "lhs_to_f64",
                )?
            };
            let r = if rhs.is_float_value() {
                rhs.into_float_value()
            } else {
                self.builder.build_signed_int_to_float(
                    rhs.into_int_value(),
                    self.context.f64_type(),
                    "rhs_to_f64",
                )?
            };
            return match op {
                BinOp::Add => Ok(self.builder.build_float_add(l, r, "fadd_tmp")?.into()),
                BinOp::Sub => Ok(self.builder.build_float_sub(l, r, "fsub_tmp")?.into()),
                BinOp::Mul => Ok(self.builder.build_float_mul(l, r, "fmul_tmp")?.into()),
                BinOp::Div => Ok(self.builder.build_float_div(l, r, "fdiv_tmp")?.into()),
                BinOp::Eq => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq_tmp")?
                    .into()),
                BinOp::Ne => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne_tmp")?
                    .into()),
                BinOp::Lt => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt_tmp")?
                    .into()),
                BinOp::Le => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle_tmp")?
                    .into()),
                BinOp::Gt => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt_tmp")?
                    .into()),
                BinOp::Ge => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge_tmp")?
                    .into()),
            };
        }

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
        if val.is_float_value() {
            let v = val.into_float_value();
            return match op {
                UnOp::Neg => Ok(self.builder.build_float_neg(v, "fneg_tmp")?.into()),
                UnOp::Not => Err(anyhow!("cannot apply logical not to float value")),
            };
        }

        let v = val.into_int_value();
        match op {
            UnOp::Not => Ok(self.builder.build_not(v, "not_tmp")?.into()),
            UnOp::Neg => Ok(self.builder.build_int_neg(v, "neg_tmp")?.into()),
        }
    }
}

fn decode_izel_string_literal(raw: &str) -> String {
    let inner = if raw.len() >= 2 {
        let first = raw.chars().next().unwrap_or('\0');
        let last = raw.chars().last().unwrap_or('\0');
        if (first == '"' && last == '"') || (first == '`' && last == '`') {
            &raw[1..raw.len() - 1]
        } else {
            raw
        }
    } else {
        raw
    };

    let mut out = String::new();
    let mut chars = inner.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('x') => {
                let mut hex = String::new();
                for _ in 0..2 {
                    if let Some(peek) = chars.peek() {
                        if peek.is_ascii_hexdigit() {
                            hex.push(*peek);
                            let _ = chars.next();
                        } else {
                            break;
                        }
                    }
                }

                if let Ok(value) = u8::from_str_radix(&hex, 16) {
                    out.push(value as char);
                } else {
                    out.push('x');
                    out.push_str(&hex);
                }
            }
            Some('u') => {
                if chars.peek() == Some(&'{') {
                    let _ = chars.next();
                    let mut hex = String::new();
                    for ch in chars.by_ref() {
                        if ch == '}' {
                            break;
                        }
                        if ch != '_' {
                            hex.push(ch);
                        }
                    }

                    if let Ok(value) = u32::from_str_radix(&hex, 16) {
                        if let Some(unicode) = char::from_u32(value) {
                            out.push(unicode);
                            continue;
                        }
                    }
                }
                out.push('u');
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    out
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
            PrimType::Str => Ok(context
                .i8_type()
                .ptr_type(inkwell::AddressSpace::from(0))
                .into()),
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
            // Execution currently consumes MIR-lowered bodies.
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
            "io_print_str" => {
                let val = function.get_nth_param(0).unwrap();
                let printf = self.get_printf()?;
                let format_str = self.builder.build_global_string_ptr("%s", "format_str")?;
                let printf_args = [format_str.as_pointer_value().into(), val.into()];
                self.builder.build_call(printf, &printf_args, "printf")?;
                self.builder.build_return(None)?;
            }
            "io_eprint_str" => {
                let val = function.get_nth_param(0).unwrap().into_pointer_value();
                let write = self.get_write()?;
                let strlen = self.get_strlen()?;

                let len_call = self.builder.build_call(strlen, &[val.into()], "strlen")?;
                let len = len_call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("strlen did not return a length"))?
                    .into_int_value();

                let stderr_fd = self.context.i32_type().const_int(2, false);
                self.builder.build_call(
                    write,
                    &[stderr_fd.into(), val.into(), len.into()],
                    "write_stderr",
                )?;

                let newline = self.builder.build_global_string_ptr("\n", "stderr_nl")?;
                let one = self.context.i64_type().const_int(1, false);
                self.builder.build_call(
                    write,
                    &[
                        stderr_fd.into(),
                        newline.as_pointer_value().into(),
                        one.into(),
                    ],
                    "write_stderr_nl",
                )?;

                self.builder.build_return(None)?;
            }
            "io_read_stdin" => {
                let rt = self.get_izel_io_read_stdin()?;
                let call = self.builder.build_call(rt, &[], "rt_read_stdin")?;
                let buf = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_read_stdin did not return a buffer"))?
                    .into_pointer_value();
                self.builder.build_return(Some(&buf))?;
            }
            "io_read_file" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_read_file()?;
                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_read_file")?;
                let buf = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_read_file did not return a buffer"))?
                    .into_pointer_value();
                self.builder.build_return(Some(&buf))?;
            }
            "io_write_file" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let content = function.get_nth_param(1).unwrap().into_pointer_value();
                let rt = self.get_izel_io_write_file()?;
                let call =
                    self.builder
                        .build_call(rt, &[path.into(), content.into()], "rt_write_file")?;
                let written = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_write_file did not return status"))?
                    .into_int_value();
                self.builder.build_return(Some(&written))?;
            }
            "io_append_file" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let content = function.get_nth_param(1).unwrap().into_pointer_value();
                let rt = self.get_izel_io_append_file()?;
                let call = self.builder.build_call(
                    rt,
                    &[path.into(), content.into()],
                    "rt_append_file",
                )?;
                let written = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_append_file did not return status"))?
                    .into_int_value();
                self.builder.build_return(Some(&written))?;
            }
            "io_remove_file" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_remove_file()?;

                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_remove_file")?;
                let status = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_remove_file did not return a status code"))?
                    .into_int_value();
                self.builder.build_return(Some(&status))?;
            }
            "io_file_exists" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_file_exists()?;

                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_file_exists")?;
                let exists = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_file_exists did not return a status code"))?
                    .into_int_value();
                self.builder.build_return(Some(&exists))?;
            }
            "io_file_exists_bool" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_file_exists()?;

                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_file_exists_bool")?;
                let exists_i32 = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_file_exists did not return a status code"))?
                    .into_int_value();
                let exists_bool = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    exists_i32,
                    self.context.i32_type().const_int(1, false),
                    "exists_bool",
                )?;
                self.builder.build_return(Some(&exists_bool))?;
            }
            "io_list_dir" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_list_dir()?;
                let call = self.builder.build_call(rt, &[path.into()], "rt_list_dir")?;
                let out_buf = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_list_dir did not return a buffer"))?
                    .into_pointer_value();
                self.builder.build_return(Some(&out_buf))?;
            }
            "io_list_dir_structured" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_list_dir_structured()?;
                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_list_dir_structured")?;
                let out_buf = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_list_dir_structured did not return a buffer"))?
                    .into_pointer_value();
                self.builder.build_return(Some(&out_buf))?;
            }
            "io_read_file_bytes_hex" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let rt = self.get_izel_io_read_file_bytes_hex()?;
                let call = self
                    .builder
                    .build_call(rt, &[path.into()], "rt_read_file_bytes_hex")?;
                let out_buf = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_read_file_bytes_hex did not return a buffer"))?
                    .into_pointer_value();
                self.builder.build_return(Some(&out_buf))?;
            }
            "io_write_file_bytes_hex" => {
                let path = function.get_nth_param(0).unwrap().into_pointer_value();
                let content_hex = function.get_nth_param(1).unwrap().into_pointer_value();
                let rt = self.get_izel_io_write_file_bytes_hex()?;
                let call = self.builder.build_call(
                    rt,
                    &[path.into(), content_hex.into()],
                    "rt_write_file_bytes_hex",
                )?;
                let written = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_write_file_bytes_hex did not return status"))?
                    .into_int_value();
                self.builder.build_return(Some(&written))?;
            }
            "io_read_stdin_int" => {
                let rt = self.get_izel_io_read_stdin_int()?;
                let call = self.builder.build_call(rt, &[], "rt_read_stdin_int")?;
                let parsed = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_read_stdin_int did not return a value"))?
                    .into_int_value();
                self.builder.build_return(Some(&parsed))?;
            }
            "io_read_stdin_float" => {
                let rt = self.get_izel_io_read_stdin_float()?;
                let call = self.builder.build_call(rt, &[], "rt_read_stdin_float")?;
                let parsed = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_read_stdin_float did not return a value"))?
                    .into_float_value();
                self.builder.build_return(Some(&parsed))?;
            }
            "io_last_status" => {
                let rt = self.get_izel_io_last_status()?;
                let call = self.builder.build_call(rt, &[], "rt_io_last_status")?;
                let status = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_last_status did not return a status"))?
                    .into_int_value();
                self.builder.build_return(Some(&status))?;
            }
            "io_last_error_kind" => {
                let rt = self.get_izel_io_last_error_kind()?;
                let call = self.builder.build_call(rt, &[], "rt_io_last_error_kind")?;
                let kind = call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("izel_io_last_error_kind did not return a kind"))?
                    .into_int_value();
                self.builder.build_return(Some(&kind))?;
            }
            "io_print_newline" => {
                let printf = self.get_printf()?;
                let format_str = self.builder.build_global_string_ptr("\n", "format_nl")?;
                let printf_args = [format_str.as_pointer_value().into()];
                self.builder.build_call(printf, &printf_args, "printf")?;
                self.builder.build_return(None)?;
            }
            "i32_to_str" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                let malloc = self.get_malloc()?;
                let snprintf = self.get_snprintf()?;

                let capacity = self.context.i64_type().const_int(32, false);
                let buf_call = self
                    .builder
                    .build_call(malloc, &[capacity.into()], "str_buf")?;
                let buf = buf_call
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow!("malloc did not return a buffer pointer"))?
                    .into_pointer_value();

                let format_str = self
                    .builder
                    .build_global_string_ptr("%d", "format_i32_to_str")?;
                let snprintf_args = [
                    buf.into(),
                    capacity.into(),
                    format_str.as_pointer_value().into(),
                    val.into(),
                ];
                self.builder
                    .build_call(snprintf, &snprintf_args, "snprintf")?;
                self.builder.build_return(Some(&buf))?;
            }
            "str_free" => {
                let ptr = function.get_nth_param(0).unwrap().into_pointer_value();
                let free = self.get_free()?;
                self.builder.build_call(free, &[ptr.into()], "free_str")?;
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
        let size_type = self.context.i64_type(); // Runtime ABI uses 64-bit size_t.
        let fn_type = ptr_type.fn_type(&[size_type.into()], false);
        Ok(self.module.add_function("malloc", fn_type, None))
    }

    fn get_strlen(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("strlen") {
            return Ok(f);
        }
        let size_type = self.context.i64_type();
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = size_type.fn_type(&[ptr_type.into()], false);
        Ok(self.module.add_function("strlen", fn_type, None))
    }

    fn get_write(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("write") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = i64_type.fn_type(&[i32_type.into(), ptr_type.into(), i64_type.into()], false);
        Ok(self.module.add_function("write", fn_type, None))
    }

    fn get_izel_io_read_stdin(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_read_stdin") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = ptr_type.fn_type(&[], false);
        Ok(self
            .module
            .add_function("izel_io_read_stdin", fn_type, None))
    }

    fn get_izel_io_read_file(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_read_file") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        Ok(self.module.add_function("izel_io_read_file", fn_type, None))
    }

    fn get_izel_io_write_file(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_write_file") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_write_file", fn_type, None))
    }

    fn get_izel_io_append_file(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_append_file") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_append_file", fn_type, None))
    }

    fn get_izel_io_remove_file(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_remove_file") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_remove_file", fn_type, None))
    }

    fn get_izel_io_file_exists(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_file_exists") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_file_exists", fn_type, None))
    }

    fn get_izel_io_list_dir(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_list_dir") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        Ok(self.module.add_function("izel_io_list_dir", fn_type, None))
    }

    fn get_izel_io_list_dir_structured(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_list_dir_structured") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_list_dir_structured", fn_type, None))
    }

    fn get_izel_io_read_file_bytes_hex(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_read_file_bytes_hex") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_read_file_bytes_hex", fn_type, None))
    }

    fn get_izel_io_write_file_bytes_hex(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_write_file_bytes_hex") {
            return Ok(f);
        }
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        Ok(self
            .module
            .add_function("izel_io_write_file_bytes_hex", fn_type, None))
    }

    fn get_izel_io_read_stdin_int(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_read_stdin_int") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        Ok(self
            .module
            .add_function("izel_io_read_stdin_int", fn_type, None))
    }

    fn get_izel_io_read_stdin_float(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_read_stdin_float") {
            return Ok(f);
        }
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[], false);
        Ok(self
            .module
            .add_function("izel_io_read_stdin_float", fn_type, None))
    }

    fn get_izel_io_last_status(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_last_status") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        Ok(self
            .module
            .add_function("izel_io_last_status", fn_type, None))
    }

    fn get_izel_io_last_error_kind(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("izel_io_last_error_kind") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        Ok(self
            .module
            .add_function("izel_io_last_error_kind", fn_type, None))
    }

    fn get_snprintf(&self) -> Result<FunctionValue<'ctx>> {
        if let Some(f) = self.module.get_function("snprintf") {
            return Ok(f);
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::from(0));
        let size_type = self.context.i64_type();
        let fn_type = i32_type.fn_type(&[ptr_type.into(), size_type.into(), ptr_type.into()], true);
        Ok(self.module.add_function("snprintf", fn_type, None))
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

        if let Some(f) = self.module.get_function("izel_io_read_stdin") {
            execution_engine.add_global_mapping(&f, izel_io_read_stdin as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_read_file") {
            execution_engine.add_global_mapping(&f, izel_io_read_file as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_write_file") {
            execution_engine.add_global_mapping(&f, izel_io_write_file as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_append_file") {
            execution_engine.add_global_mapping(&f, izel_io_append_file as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_remove_file") {
            execution_engine.add_global_mapping(&f, izel_io_remove_file as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_file_exists") {
            execution_engine.add_global_mapping(&f, izel_io_file_exists as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_list_dir") {
            execution_engine.add_global_mapping(&f, izel_io_list_dir as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_list_dir_structured") {
            execution_engine
                .add_global_mapping(&f, izel_io_list_dir_structured as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_read_file_bytes_hex") {
            execution_engine
                .add_global_mapping(&f, izel_io_read_file_bytes_hex as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_write_file_bytes_hex") {
            execution_engine
                .add_global_mapping(&f, izel_io_write_file_bytes_hex as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_read_stdin_int") {
            execution_engine.add_global_mapping(&f, izel_io_read_stdin_int as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_read_stdin_float") {
            execution_engine.add_global_mapping(&f, izel_io_read_stdin_float as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_last_status") {
            execution_engine.add_global_mapping(&f, izel_io_last_status as *const () as usize);
        }
        if let Some(f) = self.module.get_function("izel_io_last_error_kind") {
            execution_engine.add_global_mapping(&f, izel_io_last_error_kind as *const () as usize);
        }

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
    fn test_decode_izel_string_literal_handles_quotes_and_escapes() {
        let text = decode_izel_string_literal("\"line1\\nline2\\t\\x41\\u{1F600}\\\"\\\\\"");
        assert_eq!(text, "line1\nline2\tA😀\"\\");

        let raw = decode_izel_string_literal("plain\\ntext");
        assert_eq!(raw, "plain\ntext");
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
                111,
                "print_str",
                "io_print_str",
                vec![param(112, "msg", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Void),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                117,
                "eprint_str",
                "io_eprint_str",
                vec![param(118, "msg", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Void),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                119,
                "read_stdin",
                "io_read_stdin",
                vec![],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                120,
                "read_file",
                "io_read_file",
                vec![param(121, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                122,
                "write_file",
                "io_write_file",
                vec![
                    param(123, "path", Type::Prim(PrimType::Str)),
                    param(124, "content", Type::Prim(PrimType::Str)),
                ],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                125,
                "append_file",
                "io_append_file",
                vec![
                    param(126, "path", Type::Prim(PrimType::Str)),
                    param(127, "content", Type::Prim(PrimType::Str)),
                ],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                128,
                "remove_file",
                "io_remove_file",
                vec![param(129, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                130,
                "file_exists",
                "io_file_exists",
                vec![param(131, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                137,
                "file_exists_bool",
                "io_file_exists_bool",
                vec![param(138, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Bool),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                132,
                "list_dir",
                "io_list_dir",
                vec![param(133, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                139,
                "list_dir_structured",
                "io_list_dir_structured",
                vec![param(140, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                141,
                "read_file_bytes_hex",
                "io_read_file_bytes_hex",
                vec![param(142, "path", Type::Prim(PrimType::Str))],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                143,
                "write_file_bytes_hex",
                "io_write_file_bytes_hex",
                vec![
                    param(144, "path", Type::Prim(PrimType::Str)),
                    param(145, "content_hex", Type::Prim(PrimType::Str)),
                ],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                134,
                "read_stdin_int",
                "io_read_stdin_int",
                vec![],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                135,
                "read_stdin_float",
                "io_read_stdin_float",
                vec![],
                Type::Prim(PrimType::F64),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                136,
                "last_status",
                "io_last_status",
                vec![],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                146,
                "last_error_kind",
                "io_last_error_kind",
                vec![],
                Type::Prim(PrimType::I32),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                113,
                "i32_to_str",
                "i32_to_str",
                vec![param(114, "v", Type::Prim(PrimType::I32))],
                Type::Prim(PrimType::Str),
            ))),
            HirItem::Forge(Box::new(intrinsic_forge(
                115,
                "free_str",
                "str_free",
                vec![param(116, "msg", Type::Prim(PrimType::Str))],
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
        assert!(ir.contains("define void @_iz_print_str(ptr %0)"));
        assert!(ir.contains("define void @_iz_eprint_str(ptr %0)"));
        assert!(ir.contains("define ptr @_iz_read_stdin()"));
        assert!(ir.contains("define ptr @_iz_read_file(ptr %0)"));
        assert!(ir.contains("define i32 @_iz_write_file(ptr %0, ptr %1)"));
        assert!(ir.contains("define i32 @_iz_append_file(ptr %0, ptr %1)"));
        assert!(ir.contains("define i32 @_iz_remove_file(ptr %0)"));
        assert!(ir.contains("define i32 @_iz_file_exists(ptr %0)"));
        assert!(ir.contains("define i1 @_iz_file_exists_bool(ptr %0)"));
        assert!(ir.contains("define ptr @_iz_list_dir(ptr %0)"));
        assert!(ir.contains("define ptr @_iz_list_dir_structured(ptr %0)"));
        assert!(ir.contains("define ptr @_iz_read_file_bytes_hex(ptr %0)"));
        assert!(ir.contains("define i32 @_iz_write_file_bytes_hex(ptr %0, ptr %1)"));
        assert!(ir.contains("define i32 @_iz_read_stdin_int()"));
        assert!(ir.contains("define double @_iz_read_stdin_float()"));
        assert!(ir.contains("define i32 @_iz_last_status()"));
        assert!(ir.contains("define i32 @_iz_last_error_kind()"));
        assert!(ir.contains("define ptr @_iz_i32_to_str(i32 %0)"));
        assert!(ir.contains("define void @_iz_free_str(ptr %0)"));
        assert!(ir.contains("declare i32 @printf(ptr, ...)"));
        assert!(ir.contains("declare i32 @izel_io_last_status()"));
        assert!(ir.contains("declare i32 @izel_io_last_error_kind()"));
        assert!(ir.contains("declare ptr @izel_io_read_stdin()"));
        assert!(ir.contains("declare ptr @izel_io_read_file(ptr)"));
        assert!(ir.contains("declare i32 @izel_io_write_file(ptr, ptr)"));
        assert!(ir.contains("declare i32 @izel_io_append_file(ptr, ptr)"));
        assert!(ir.contains("declare i32 @izel_io_remove_file(ptr)"));
        assert!(ir.contains("declare i32 @izel_io_file_exists(ptr)"));
        assert!(ir.contains("declare ptr @izel_io_list_dir(ptr)"));
        assert!(ir.contains("declare ptr @izel_io_list_dir_structured(ptr)"));
        assert!(ir.contains("declare ptr @izel_io_read_file_bytes_hex(ptr)"));
        assert!(ir.contains("declare i32 @izel_io_write_file_bytes_hex(ptr, ptr)"));
        assert!(ir.contains("declare i32 @izel_io_read_stdin_int()"));
        assert!(ir.contains("declare double @izel_io_read_stdin_float()"));
        assert!(ir.contains("declare i64 @strlen(ptr)"));
        assert!(ir.contains("declare i64 @write(i32, ptr, i64)"));
        assert!(ir.contains("declare i32 @snprintf(ptr, i64, ptr, ...)"));
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

        let snprintf1 = codegen.get_snprintf()?;
        let snprintf2 = codegen.get_snprintf()?;
        assert_eq!(
            snprintf1.as_global_value().as_pointer_value(),
            snprintf2.as_global_value().as_pointer_value()
        );

        let strlen1 = codegen.get_strlen()?;
        let strlen2 = codegen.get_strlen()?;
        assert_eq!(
            strlen1.as_global_value().as_pointer_value(),
            strlen2.as_global_value().as_pointer_value()
        );

        let write1 = codegen.get_write()?;
        let write2 = codegen.get_write()?;
        assert_eq!(
            write1.as_global_value().as_pointer_value(),
            write2.as_global_value().as_pointer_value()
        );

        let rt_read_stdin1 = codegen.get_izel_io_read_stdin()?;
        let rt_read_stdin2 = codegen.get_izel_io_read_stdin()?;
        assert_eq!(
            rt_read_stdin1.as_global_value().as_pointer_value(),
            rt_read_stdin2.as_global_value().as_pointer_value()
        );

        let rt_read_file1 = codegen.get_izel_io_read_file()?;
        let rt_read_file2 = codegen.get_izel_io_read_file()?;
        assert_eq!(
            rt_read_file1.as_global_value().as_pointer_value(),
            rt_read_file2.as_global_value().as_pointer_value()
        );

        let rt_write_file1 = codegen.get_izel_io_write_file()?;
        let rt_write_file2 = codegen.get_izel_io_write_file()?;
        assert_eq!(
            rt_write_file1.as_global_value().as_pointer_value(),
            rt_write_file2.as_global_value().as_pointer_value()
        );

        let rt_append_file1 = codegen.get_izel_io_append_file()?;
        let rt_append_file2 = codegen.get_izel_io_append_file()?;
        assert_eq!(
            rt_append_file1.as_global_value().as_pointer_value(),
            rt_append_file2.as_global_value().as_pointer_value()
        );

        let rt_remove_file1 = codegen.get_izel_io_remove_file()?;
        let rt_remove_file2 = codegen.get_izel_io_remove_file()?;
        assert_eq!(
            rt_remove_file1.as_global_value().as_pointer_value(),
            rt_remove_file2.as_global_value().as_pointer_value()
        );

        let rt_exists1 = codegen.get_izel_io_file_exists()?;
        let rt_exists2 = codegen.get_izel_io_file_exists()?;
        assert_eq!(
            rt_exists1.as_global_value().as_pointer_value(),
            rt_exists2.as_global_value().as_pointer_value()
        );

        let rt_list_dir1 = codegen.get_izel_io_list_dir()?;
        let rt_list_dir2 = codegen.get_izel_io_list_dir()?;
        assert_eq!(
            rt_list_dir1.as_global_value().as_pointer_value(),
            rt_list_dir2.as_global_value().as_pointer_value()
        );

        let rt_list_dir_structured1 = codegen.get_izel_io_list_dir_structured()?;
        let rt_list_dir_structured2 = codegen.get_izel_io_list_dir_structured()?;
        assert_eq!(
            rt_list_dir_structured1.as_global_value().as_pointer_value(),
            rt_list_dir_structured2.as_global_value().as_pointer_value()
        );

        let rt_read_bytes_hex1 = codegen.get_izel_io_read_file_bytes_hex()?;
        let rt_read_bytes_hex2 = codegen.get_izel_io_read_file_bytes_hex()?;
        assert_eq!(
            rt_read_bytes_hex1.as_global_value().as_pointer_value(),
            rt_read_bytes_hex2.as_global_value().as_pointer_value()
        );

        let rt_write_bytes_hex1 = codegen.get_izel_io_write_file_bytes_hex()?;
        let rt_write_bytes_hex2 = codegen.get_izel_io_write_file_bytes_hex()?;
        assert_eq!(
            rt_write_bytes_hex1.as_global_value().as_pointer_value(),
            rt_write_bytes_hex2.as_global_value().as_pointer_value()
        );

        let rt_read_stdin_int1 = codegen.get_izel_io_read_stdin_int()?;
        let rt_read_stdin_int2 = codegen.get_izel_io_read_stdin_int()?;
        assert_eq!(
            rt_read_stdin_int1.as_global_value().as_pointer_value(),
            rt_read_stdin_int2.as_global_value().as_pointer_value()
        );

        let rt_read_stdin_float1 = codegen.get_izel_io_read_stdin_float()?;
        let rt_read_stdin_float2 = codegen.get_izel_io_read_stdin_float()?;
        assert_eq!(
            rt_read_stdin_float1.as_global_value().as_pointer_value(),
            rt_read_stdin_float2.as_global_value().as_pointer_value()
        );

        let rt_last_status1 = codegen.get_izel_io_last_status()?;
        let rt_last_status2 = codegen.get_izel_io_last_status()?;
        assert_eq!(
            rt_last_status1.as_global_value().as_pointer_value(),
            rt_last_status2.as_global_value().as_pointer_value()
        );

        let rt_last_error_kind1 = codegen.get_izel_io_last_error_kind()?;
        let rt_last_error_kind2 = codegen.get_izel_io_last_error_kind()?;
        assert_eq!(
            rt_last_error_kind1.as_global_value().as_pointer_value(),
            rt_last_error_kind2.as_global_value().as_pointer_value()
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

        let str_const = mir_codegen
            .gen_operand(&Operand::Constant(Constant::Str("x".to_string())), &body)
            .expect("string constants should lower to global pointer values");
        assert!(matches!(str_const, BasicValueEnum::PointerValue(_)));

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
    fn test_mir_codegen_phi_with_incoming_entries() -> Result<()> {
        use izel_mir::{BasicBlock, LocalData};

        let context = Context::create();
        let module = context.create_module("test_phi_codegen");
        let builder = context.create_builder();
        let function = module.add_function("phi_fn", context.i32_type().fn_type(&[], false), None);

        let mut body = MirBody::new();
        let pred_with_term = body.blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: Some(Terminator::Goto(body.entry)),
        });
        let pred_without_term = body.blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: None,
        });
        let target = body.blocks.add_node(BasicBlock {
            instructions: Vec::new(),
            terminator: Some(Terminator::Return(Some(Operand::Copy(Local(1))))),
        });

        body.locals = vec![
            LocalData {
                name: "a".into(),
                ty: Type::Prim(PrimType::I32),
            },
            LocalData {
                name: "b".into(),
                ty: Type::Prim(PrimType::I32),
            },
        ];

        let entry_bb = context.append_basic_block(function, "entry");
        let pred_term_bb = context.append_basic_block(function, "pred_term");
        let pred_none_bb = context.append_basic_block(function, "pred_none");
        let target_bb = context.append_basic_block(function, "target");

        let mut mir_codegen = MirCodegen::new(&context, &module, &builder);
        mir_codegen.blocks.insert(body.entry, entry_bb);
        mir_codegen.blocks.insert(pred_with_term, pred_term_bb);
        mir_codegen.blocks.insert(pred_without_term, pred_none_bb);
        mir_codegen.blocks.insert(target, target_bb);

        builder.position_at_end(entry_bb);
        let a_ptr = builder.build_alloca(context.i32_type(), "a")?;
        let b_ptr = builder.build_alloca(context.i32_type(), "b")?;
        builder.build_store(a_ptr, context.i32_type().const_int(7, false))?;
        builder.build_store(b_ptr, context.i32_type().const_int(0, false))?;
        builder.build_unconditional_branch(target_bb)?;

        builder.position_at_end(pred_term_bb);
        builder.build_unconditional_branch(target_bb)?;

        builder.position_at_end(target_bb);
        mir_codegen.locals.insert(Local(0), a_ptr);
        mir_codegen.locals.insert(Local(1), b_ptr);

        let phi_inst = Instruction::Phi(
            Local(1),
            vec![(pred_with_term, Local(0)), (pred_without_term, Local(0))],
        );
        mir_codegen.gen_instruction(&phi_inst, &body)?;
        mir_codegen.gen_terminator(&Terminator::Return(Some(Operand::Copy(Local(1)))), &body)?;

        let ir = module.print_to_string().to_string();
        assert!(ir.contains("phi i32"));
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

        let str_ptr = llvm_type_static(&context, &Type::Prim(PrimType::Str))?;
        assert!(matches!(
            str_ptr,
            inkwell::types::BasicTypeEnum::PointerType(_)
        ));

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
        let io_print_str = HirItem::Forge(Box::new(intrinsic_forge(
            907,
            "print_str",
            "io_print_str",
            vec![param(908, "msg", Type::Prim(PrimType::Str))],
            Type::Prim(PrimType::Void),
        )));
        let io_eprint_str = HirItem::Forge(Box::new(intrinsic_forge(
            918,
            "eprint_str",
            "io_eprint_str",
            vec![param(919, "msg", Type::Prim(PrimType::Str))],
            Type::Prim(PrimType::Void),
        )));
        let i32_to_str = HirItem::Forge(Box::new(intrinsic_forge(
            909,
            "i32_to_str",
            "i32_to_str",
            vec![param(910, "v", Type::Prim(PrimType::I32))],
            Type::Prim(PrimType::Str),
        )));
        let str_free = HirItem::Forge(Box::new(intrinsic_forge(
            916,
            "free_str",
            "str_free",
            vec![param(917, "msg", Type::Prim(PrimType::Str))],
            Type::Prim(PrimType::Void),
        )));
        let simd_sum = HirItem::Forge(Box::new(intrinsic_forge(
            911,
            "simd_sum",
            "simd_i32x4_sum",
            vec![
                param(912, "a", Type::Prim(PrimType::I32)),
                param(913, "b", Type::Prim(PrimType::I32)),
                param(914, "c", Type::Prim(PrimType::I32)),
                param(915, "d", Type::Prim(PrimType::I32)),
            ],
            Type::Prim(PrimType::I32),
        )));

        let items = [
            i32_abs,
            io_print_int,
            io_print_newline,
            io_print_str,
            io_eprint_str,
            i32_to_str,
            str_free,
            simd_sum,
        ];
        for item in &items {
            codegen.declare_item(item)?;
            codegen.gen_item(item, &HashMap::new())?;
        }

        let ir = codegen.emit_llvm_ir();
        assert!(ir.contains("@llvm.abs.i32"));
        assert!(ir.contains("@printf"));
        assert!(ir.contains("@write"));
        assert!(ir.contains("@strlen"));
        assert!(ir.contains("@snprintf"));
        assert!(ir.contains("@llvm.vector.reduce.add.v4i32"));
        Ok(())
    }
}
