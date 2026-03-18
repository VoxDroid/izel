//! LLVM code generation for Izel.

use anyhow::{anyhow, Result};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;
use inkwell::values::{FunctionValue, PointerValue};
use izel_parser::ast;
use std::collections::HashMap;

pub struct Codegen<'ctx, 'a> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub source: &'a str,
    pub variables: HashMap<String, PointerValue<'ctx>>,
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
                if let Some(ast::Expr::Literal(ast::Literal::Str(name))) = attr.args.get(0) {
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

    fn gen_intrinsic_body(&mut self, function: FunctionValue<'ctx>, name: &str, _params: &[ast::Param]) -> Result<()> {
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        match name {
            "i32_abs" => {
                let val = function.get_nth_param(0).unwrap().into_int_value();
                // LLVM doesn't have a direct 'abs' int instruction, we use the intrinsic
                let abs_intrinsic = self.get_intrinsic("llvm.abs.i32")?;
                let call = self.builder.build_call(abs_intrinsic, &[val.into(), self.context.bool_type().const_int(0, false).into()], "abs")?;
                self.builder.build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_sqrt" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let sqrt_intrinsic = self.get_intrinsic("llvm.sqrt.f64")?;
                let call = self.builder.build_call(sqrt_intrinsic, &[val.into()], "sqrt")?;
                self.builder.build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_ceil" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let ceil_intrinsic = self.get_intrinsic("llvm.ceil.f64")?;
                let call = self.builder.build_call(ceil_intrinsic, &[val.into()], "ceil")?;
                self.builder.build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
            }
            "f64_floor" => {
                let val = function.get_nth_param(0).unwrap().into_float_value();
                let floor_intrinsic = self.get_intrinsic("llvm.floor.f64")?;
                let call = self.builder.build_call(floor_intrinsic, &[val.into()], "floor")?;
                self.builder.build_return(Some(&call.try_as_basic_value().left().unwrap()))?;
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
            generic_params: vec![],
            params: vec![ast::Param {
                name: "val".to_string(),
                ty: ast::Type::Prim("i32".to_string()),
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("i32".to_string()),
            effects: vec![],
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
            generic_params: vec![],
            params: vec![ast::Param {
                name: "b".to_string(),
                ty: ast::Type::Prim("bool".to_string()),
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("bool".to_string()),
            effects: vec![],
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str("bool_not".to_string()))],
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
            generic_params: vec![],
            params: vec![ast::Param {
                name: "v".to_string(),
                ty: ast::Type::Prim("f64".to_string()),
                span: Span::dummy(),
            }],
            ret_type: ast::Type::Prim("f64".to_string()),
            effects: vec![],
            attributes: vec![ast::Attribute {
                name: "intrinsic".to_string(),
                args: vec![ast::Expr::Literal(ast::Literal::Str("f64_sqrt".to_string()))],
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
}
