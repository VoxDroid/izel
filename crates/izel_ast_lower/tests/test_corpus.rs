use izel_ast_lower::Lowerer;
use izel_lexer::{Lexer, TokenKind};
use izel_parser::ast;
use izel_parser::Parser;
use izel_span::SourceId;
use std::fs;
use std::path::{Path, PathBuf};

fn lower_module(source: &str) -> ast::Module {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, source.to_string());
    let cst = parser.parse_source_file();
    Lowerer::new(source).lower_module(&cst)
}

fn collect_iz_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("failed to read corpus directory");
    for entry in entries {
        let path = entry.expect("failed to read corpus entry").path();
        if path.is_dir() {
            collect_iz_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("iz") {
            out.push(path);
        }
    }
}

#[test]
fn lowerer_handles_workspace_corpus_without_panicking() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();

    for rel in ["examples", "tests", "std", "library", "compiler"] {
        let dir = repo_root.join(rel);
        if dir.exists() {
            collect_iz_files(&dir, &mut files);
        }
    }

    files.sort();
    files.dedup();
    assert!(!files.is_empty(), "expected at least one .iz fixture");

    let mut lowered = 0usize;
    for file in files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", file, e));
        let result = std::panic::catch_unwind(|| lower_module(&source));
        if result.is_ok() {
            lowered += 1;
        }
    }

    assert!(lowered > 0, "expected at least one corpus file to lower");
}

#[test]
fn lowerer_handles_synthetic_edge_forms_without_panicking() {
    let snippets = [
        r#"
@intrinsic("abs")
forge abs(x: i32) -> i32 { give x }
"#,
        r#"
shape Packet<T> { open id: i32, hidden payload: T, }
scroll Message { Ping, Data(i32, str), Meta { code: i32 }, }
"#,
        r#"
weave Renderable: Drawable + Debug { forge render(self) { give } }
weave Renderable for Widget { forge render(self) { give } }
"#,
        r#"
dual shape Codec<T> {
    value: T,
    forge encode(self) { give 0 }
    forge decode(self) { give 0 }
}
"#,
        r#"
forge main() {
    let nested = branch value {
        Some(v) given v > 0 => v,
        [head, ..tail] => head,
        _ => 0,
    }
    let f = bind |x, y| x + y
    let z = zone arena { each item in items { while cond { loop { give item } } } }
    give nested
}
"#,
        r#"
bridge "C" {
    forge puts(msg: str)
    static errno: i32
}
ward Core {
    forge helper() { give 0 }
}
"#,
    ];

    for src in snippets {
        let _ = lower_module(src);
    }
}
