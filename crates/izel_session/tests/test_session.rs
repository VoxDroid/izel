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
        "--target",
        "x86_64-unknown-linux-gnu",
        "--emit",
        "mir",
        "-O",
        "2",
        "--debug",
        "--no-std",
        "--check-effects",
        "--run",
        "--check-contracts",
        "--keep-witnesses",
        "--lto",
        "--strip",
        "--target-cpu",
        "native",
        "--error-format",
        "json",
        "--edition",
        "2026",
    ]);

    assert_eq!(opts.input, Some(PathBuf::from("examples/hello.iz")));
    assert_eq!(opts.output, Some(PathBuf::from("target/hello")));
    assert_eq!(opts.target.as_deref(), Some("x86_64-unknown-linux-gnu"));
    assert_eq!(opts.emit.as_deref(), Some("mir"));
    assert_eq!(opts.opt, "2");
    assert!(opts.debug);
    assert!(opts.no_std);
    assert!(opts.check_effects);
    assert!(opts.run);
    assert!(opts.check_contracts);
    assert!(opts.keep_witnesses);
    assert!(opts.lto);
    assert!(opts.strip);
    assert_eq!(opts.target_cpu.as_deref(), Some("native"));
    assert_eq!(opts.error_format.as_deref(), Some("json"));
    assert_eq!(opts.edition, "2026");
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
        target: Some("wasm32-unknown-unknown".to_string()),
        emit: Some("llvm-ir".to_string()),
        opt: "3".to_string(),
        debug: false,
        no_std: false,
        check_effects: true,
        run: false,
        check_contracts: true,
        keep_witnesses: false,
        lto: true,
        strip: false,
        target_cpu: Some("native".to_string()),
        error_format: Some("human".to_string()),
        edition: "2025".to_string(),
        command: Some(Command::Lsp),
    };

    let session = Session::new(options);
    assert_eq!(
        session.options.target.as_deref(),
        Some("wasm32-unknown-unknown")
    );
    assert_eq!(session.options.emit.as_deref(), Some("llvm-ir"));
    assert_eq!(session.options.opt, "3");
    assert!(session.options.check_effects);
    assert!(session.options.check_contracts);
    assert!(session.options.lto);
    assert_eq!(session.options.target_cpu.as_deref(), Some("native"));
    assert_eq!(session.options.error_format.as_deref(), Some("human"));
    assert_eq!(session.options.edition, "2025");
    assert!(matches!(session.options.command, Some(Command::Lsp)));
}
