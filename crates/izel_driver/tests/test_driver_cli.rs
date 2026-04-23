use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_file(ext: &str, content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "izel-driver-test-{}-{}-{}.{}",
        std::process::id(),
        nonce,
        serial,
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

fn run_izelc_with_null_stdin(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izelc"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("failed to execute izelc")
}

fn collect_iz_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("failed to read directory");
    for entry in entries {
        let path = entry.expect("failed to read directory entry").path();
        if path.is_dir() {
            collect_iz_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("iz") {
            out.push(path);
        }
    }
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
fn cli_fmt_subcommand_reports_missing_file_error() {
    let mut missing = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    missing.push(format!("izel-driver-missing-{}-{}.iz", nonce, serial));
    let missing_arg = missing.to_string_lossy().to_string();

    let output = run_izelc(&["fmt", &missing_arg]);

    assert!(!output.status.success(), "fmt should fail for missing file");
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("No such file") || combined.contains("os error"),
        "missing-file error was not surfaced: {combined}"
    );
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
fn cli_deps_subcommand_fails_for_missing_manifest() {
    let mut missing = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    missing.push(format!("izel-driver-missing-{}-{}.toml", nonce, serial));
    let missing_arg = missing.to_string_lossy().to_string();

    let output = run_izelc(&["deps", &missing_arg]);

    assert!(
        !output.status.success(),
        "deps should fail for missing manifest path"
    );
}

#[test]
fn cli_deps_subcommand_fails_for_invalid_manifest() {
    let manifest = temp_file(
        "toml",
        r#"[package]
name = "broken"
version =
"#,
    );
    let manifest_arg = manifest.to_string_lossy().to_string();

    let output = run_izelc(&["deps", &manifest_arg]);
    let _ = fs::remove_file(&manifest);

    assert!(
        !output.status.success(),
        "deps should fail for malformed manifest"
    );

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("failed")
            || combined.contains("Failed")
            || combined.contains("error")
            || combined.contains("Error")
            || combined.contains("invalid"),
        "invalid-manifest error was not surfaced: {combined}"
    );
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
fn cli_compile_with_run_flag_executes_jit_path() {
    let input = temp_file("iz", "forge main() { give 0 }");
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- JIT Execution ---"));
    assert!(stdout.contains("JIT Exit Code:"));
}

#[test]
fn cli_compile_with_run_flag_normalizes_string_escapes() {
    let input = temp_file(
        "iz",
        r#"@intrinsic("io_print_str")
forge print(msg: str)

forge main() -> int {
    print("line1\nline2\t\x41")
    give 0
}"#,
    );
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("line1\nline2\tA"));
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
fn cli_lsp_subcommand_is_reachable_in_test_mode() {
    let output = run_izelc_with_null_stdin(&["lsp"]);
    assert!(
        output.status.success(),
        "lsp command failed with closed stdin: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

#[test]
fn cli_compilation_corpus_does_not_panic_or_crash() {
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
    assert!(!files.is_empty(), "expected at least one .iz corpus file");

    for path in files {
        let arg = path.to_string_lossy().to_string();
        let output = run_izelc(&[&arg]);

        assert!(
            output.status.code().is_some(),
            "izelc terminated by signal for {}",
            arg
        );

        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !combined.contains("thread 'main' panicked"),
            "izelc panicked while compiling {}\n{}",
            arg,
            combined
        );
    }
}
