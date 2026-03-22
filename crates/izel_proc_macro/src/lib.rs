use izel_lexer::Token;

/// A stream of tokens for procedural macro processing.
#[derive(Debug, Clone, Default)]
pub struct TokenStream {
    pub tokens: Vec<Token>,
}

impl TokenStream {
    pub fn new() -> Self {
        Self { tokens: Vec::new() }
    }
}

/// The procedural macro trait.
pub trait ProcMacro {
    fn name(&self) -> &str;
    fn expand(&self, input: TokenStream) -> TokenStream;
}

/// A derived procedural macro (e.g. @derive(Show)).
pub trait DeriveMacro: ProcMacro {
    fn expand_derive(&self, input: TokenStream) -> TokenStream {
        self.expand(input)
    }
}
