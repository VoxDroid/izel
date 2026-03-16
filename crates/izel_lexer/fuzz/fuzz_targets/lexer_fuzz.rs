#![no_main]

use libfuzzer_sys::fuzz_target;
use izel_lexer::Lexer;
use izel_span::SourceId;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut lexer = Lexer::new(s, SourceId(0));
        loop {
            let token = lexer.next_token();
            if token.kind == izel_lexer::TokenKind::Eof {
                break;
            }
        }
    }
});
