//! LLVM code generation for Izel.

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{FunctionValue, PointerValue};
use izel_parser::cst::{SyntaxNode, SyntaxElement, NodeKind};
use izel_lexer::TokenKind;
use anyhow::{Result, anyhow};

pub struct Codegen<'ctx, 'a> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub source: &'a str,
}

impl<'ctx, 'a> Codegen<'ctx, 'a> {
    pub fn new(context: &'ctx Context, name: &str, source: &'a str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        Self { context, module, builder, source }
    }

    pub fn gen_source_file(&mut self, node: &SyntaxNode) -> Result<()> {
        if node.kind != NodeKind::SourceFile {
            return Err(anyhow!("Expected SourceFile node"));
        }

        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                match child_node.kind {
                    NodeKind::ForgeDecl => {
                        self.gen_forge_decl(child_node)?;
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn gen_forge_decl(&mut self, node: &SyntaxNode) -> Result<FunctionValue<'ctx>> {
        let mut name = None;
        
        for child in &node.children {
            if let SyntaxElement::Token(token) = child {
                if token.kind == TokenKind::Ident {
                    let span = token.span;
                    name = Some(self.source[span.lo.0 as usize..span.hi.0 as usize].to_string());
                    break;
                }
            }
        }
        
        let name = name.ok_or_else(|| anyhow!("Forge declaration missing name"))?;

        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function(&name, fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Minimal body: return 0
        self.builder.build_return(Some(&i32_type.const_int(0, false)))?;

        Ok(function)
    }

    pub fn emit_llvm_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    pub fn run_jit(&self) -> Result<i32> {
        let execution_engine = self.module.create_jit_execution_engine(inkwell::OptimizationLevel::None)
            .map_err(|e| anyhow!("Failed to create JIT: {:?}", e))?;
        
        unsafe {
            let main_fn = execution_engine.get_function::<unsafe extern "C" fn() -> i32>("main")
                .map_err(|e| anyhow!("Failed to find main function in JIT: {:?}", e))?;
            
            println!("--- JIT Execution ---");
            let result = main_fn.call();
            println!("JIT Exit Code: {}", result);
            println!("----------------------\n");
            
            Ok(result)
        }
    }
}
