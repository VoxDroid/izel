use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_file(ext: &str, content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    path.push(format!(
        "izel-driver-test-{}-{}.{}",
        std::process::id(),
        nonce,
        ext
    ));
    fs::write(&path, content).expect("failed to write temp fixture");
    path
}

fn run_izelc(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izelc"))
        .args(args)
        .output()
        .expect("failed to execute izelc")
}

#[test]
fn cli_fmt_subcommand_formats_source_file() {
    let input = temp_file("iz", "forge main() { give 0 }");
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc(&["fmt", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "fmt command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Izel Compiler"));
    assert!(stdout.contains("forge main()"));
}

#[test]
fn cli_deps_subcommand_loads_manifest() {
    let manifest = temp_file(
        "toml",
        r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
"#,
    );
    let manifest_arg = manifest.to_string_lossy().to_string();

    let output = run_izelc(&["deps", &manifest_arg]);
    let _ = fs::remove_file(&manifest);

    assert!(
        output.status.success(),
        "deps command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Loaded manifest for package: demo v0.1.0"));
}

#[test]
fn cli_compile_path_emits_llvm_ir_for_valid_source() {
    let input = temp_file("iz", "forge main() { give 0 }");
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc(&[&input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Type checking..."));
    assert!(stdout.contains("Generating LLVM IR..."));
    assert!(stdout.contains("--- LLVM IR ---"));
}

#[test]
fn cli_compile_path_surfaces_type_errors() {
    let input = temp_file("iz", "forge main() -> i32 { true }");
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc(&[&input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        !output.status.success(),
        "type error should fail compilation"
    );

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("Block return type mismatch"));
}

#[test]
fn cli_without_input_reports_required_input_message() {
    let output = run_izelc(&[]);

    assert!(!output.status.success());
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("Input file required for compilation"));
}
