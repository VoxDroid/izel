use izel_parser::ast;

/// Generates documentation from the Izel AST.
pub struct DocGenerator {
    pub output: String,
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DocGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn generate(&mut self, module: &ast::Module) -> String {
        self.output.clear();
        self.output.push_str("# Module Documentation\n\n");
        for item in &module.items {
            self.generate_item(item);
        }
        self.output.clone()
    }

    fn generate_item(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Forge(f) => {
                self.output.push_str(&format!("## Forge: {}\n", f.name));
                for attr in &f.attributes {
                    if let Some(doc) = self.extract_doc(attr) {
                        self.output.push_str(&format!("{}\n", doc));
                    }
                }
                self.output.push('\n');
            }
            ast::Item::Shape(s) => {
                self.output.push_str(&format!("## Shape: {}\n\n", s.name));
            }
            _ => {}
        }
    }

    fn extract_doc(&self, attr: &ast::Attribute) -> Option<String> {
        if attr.name == "doc" {
            if let Some(ast::Expr::Literal(ast::Literal::Str(s))) = attr.args.first() {
                return Some(s.clone());
            }
        }
        None
    }
}
