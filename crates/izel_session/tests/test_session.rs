use clap::Parser;
use izel_session::{Command, Session, SessionOptions};
use std::path::PathBuf;

#[test]
fn parse_compiler_flags_for_direct_compilation_mode() {
    let opts = SessionOptions::parse_from([
        "izelc",
        "examples/hello.iz",
        "-o",
        "target/hello",
        "--emit",
        "mir",
        "-O",
        "2",
        "--run",
        "--check-contracts",
    ]);

    assert_eq!(opts.input, Some(PathBuf::from("examples/hello.iz")));
    assert_eq!(opts.output, Some(PathBuf::from("target/hello")));
    assert_eq!(opts.emit.as_deref(), Some("mir"));
    assert_eq!(opts.opt, "2");
    assert!(opts.run);
    assert!(opts.check_contracts);
    assert!(opts.command.is_none());
}

#[test]
fn parse_fmt_subcommand() {
    let opts = SessionOptions::parse_from(["izelc", "fmt", "examples/hello.iz"]);

    match opts.command {
        Some(Command::Fmt { input }) => {
            assert_eq!(input, PathBuf::from("examples/hello.iz"));
        }
        other => panic!("expected fmt subcommand, got {:?}", other),
    }
}

#[test]
fn session_wraps_options_without_mutation() {
    let options = SessionOptions {
        input: Some(PathBuf::from("examples/hello.iz")),
        output: None,
        emit: Some("llvm-ir".to_string()),
        opt: "3".to_string(),
        run: false,
        check_contracts: true,
        command: Some(Command::Lsp),
    };

    let session = Session::new(options);
    assert_eq!(session.options.emit.as_deref(), Some("llvm-ir"));
    assert_eq!(session.options.opt, "3");
    assert!(session.options.check_contracts);
    assert!(matches!(session.options.command, Some(Command::Lsp)));
}
