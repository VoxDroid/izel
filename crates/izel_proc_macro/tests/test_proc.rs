use izel_proc_macro::{DeriveMacro, ProcMacro, TokenStream};

#[derive(Default)]
struct IdentityMacro;

impl ProcMacro for IdentityMacro {
    fn name(&self) -> &str {
        "identity_macro"
    }

    fn expand(&self, input: TokenStream) -> TokenStream {
        input
    }
}

impl DeriveMacro for IdentityMacro {}

#[test]
fn token_stream_new_starts_empty() {
    let stream = TokenStream::new();
    assert!(stream.tokens.is_empty());
}

#[test]
fn proc_macro_expand_can_transform_input_stream() {
    let macro_impl = IdentityMacro;
    let input = TokenStream::new();

    let expanded = macro_impl.expand(input);
    assert!(expanded.tokens.is_empty());
}

#[test]
fn derive_macro_default_expand_delegates_to_expand() {
    let macro_impl = IdentityMacro;
    let input = TokenStream::default();
    let expanded = macro_impl.expand_derive(input);

    assert!(expanded.tokens.is_empty());
}
