use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_izel_pm(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izel"))
        .env("IZEL_PM_DRY_RUN", "1")
        .args(args)
        .output()
        .expect("failed to execute izel_pm")
}

fn run_izel_pm_in(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izel"))
        .env("IZEL_PM_DRY_RUN", "1")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to execute izel_pm")
}

fn unique_temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    path.push(format!(
        "izel-pm-cli-test-{}-{}-{}",
        std::process::id(),
        label,
        nonce
    ));
    path
}

#[test]
fn cli_help_and_build_paths_are_callable() {
    let help = run_izel_pm(&[]);
    assert!(
        help.status.success(),
        "help command failed: {}",
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(String::from_utf8_lossy(&help.stdout).contains("izel new <name>"));

    let build = run_izel_pm(&["build"]);
    assert!(
        build.status.success(),
        "build command failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let build_out = String::from_utf8_lossy(&build.stdout);
    assert!(build_out.contains("DRY-RUN: cargo build"));
    assert!(build_out.contains("Build finished."));
}

#[test]
fn cli_run_paths_cover_empty_and_forwarded_args() {
    let run_empty = run_izel_pm(&["run"]);
    assert!(
        run_empty.status.success(),
        "run command failed: {}",
        String::from_utf8_lossy(&run_empty.stderr)
    );
    let run_empty_out = String::from_utf8_lossy(&run_empty.stdout);
    assert!(run_empty_out.contains("DRY-RUN: cargo run"));
    assert!(run_empty_out.contains("Run finished."));

    let run_args = run_izel_pm(&["run", "--", "alpha", "beta"]);
    assert!(
        run_args.status.success(),
        "run with args failed: {}",
        String::from_utf8_lossy(&run_args.stderr)
    );
    let run_args_out = String::from_utf8_lossy(&run_args.stdout);
    assert!(run_args_out.contains("DRY-RUN: cargo run -- alpha beta"));
    assert!(run_args_out.contains("Run finished."));
}

#[test]
fn cli_new_command_creates_project_files() {
    let root = unique_temp_path("new-success");
    let root_arg = root.to_string_lossy().to_string();

    let output = run_izel_pm(&["new", &root_arg]);

    assert!(
        output.status.success(),
        "new command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Created Izel project"));

    assert!(root.join("Izel.toml").exists());
    assert!(root.join("src/main.iz").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cli_new_command_reports_creation_errors() {
    let output = run_izel_pm(&["new", "/dev/null/izel-pm-nope"]);

    assert!(
        !output.status.success(),
        "new should fail when target root is invalid"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to create project"));
}

#[test]
fn cli_unknown_command_returns_error_and_usage() {
    let output = run_izel_pm(&["deploy"]);

    assert!(!output.status.success(), "unknown command should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown command: deploy"));
    assert!(stderr.contains("izel run [-- <args>]"));
}

#[test]
fn cli_all_subcommand_success_paths_are_callable() {
    let root = unique_temp_path("new-kinds");
    let root_lib = unique_temp_path("new-lib");
    let root_ws = unique_temp_path("new-workspace");
    let root_arg = root.to_string_lossy().to_string();
    let root_lib_arg = root_lib.to_string_lossy().to_string();
    let root_ws_arg = root_ws.to_string_lossy().to_string();

    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["build", "--release", "--target", "wasm32-unknown-unknown"],
            "Build finished.",
        ),
        (vec!["test"], "Tests finished."),
        (vec!["test", "lint", "--threads", "2"], "Tests finished."),
        (vec!["bench", "pipeline"], "Bench run finished."),
        (vec!["check"], "Check finished."),
        (vec!["fmt"], "Format task finished."),
        (vec!["fmt", "--check"], "Format task finished."),
        (vec!["lint"], "Lint finished."),
        (vec!["doc"], "Documentation generated."),
        (vec!["doc", "--open"], "Documentation generated."),
        (vec!["update"], "Update finished."),
        (vec!["publish"], "Publish dry-run finished."),
        (vec!["clean"], "Clean finished."),
        (vec!["tree"], "Dependency tree finished."),
        (vec!["new", &root_arg, "--bin"], "Created Izel project"),
        (vec!["new", &root_lib_arg, "--lib"], "Created Izel project"),
        (
            vec!["new", &root_ws_arg, "--workspace"],
            "Created Izel project",
        ),
    ];

    for (args, needle) in cases {
        let output = run_izel_pm(&args);
        assert!(
            output.status.success(),
            "command {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(needle),
            "command {:?} missing output marker '{}'",
            args,
            needle
        );
        if args[0] != "new" {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("DRY-RUN: cargo"),
                "command {:?} should execute via cargo in dry-run mode",
                args
            );
        }
    }

    assert!(root.join("src/main.iz").exists());
    assert!(root_lib.join("src/lib.iz").exists());
    assert!(root_ws.join("Izel.toml").exists());

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&root_lib);
    let _ = fs::remove_dir_all(&root_ws);
}

#[test]
fn cli_add_and_remove_update_manifest_dependencies() {
    let root = unique_temp_path("manifest-edit");
    fs::create_dir_all(&root).expect("temp root should be created");
    let manifest = root.join("Izel.toml");
    fs::write(
        &manifest,
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n\n[dependencies]\nstd = \"1.0.0\"\n",
    )
    .expect("manifest should be written");

    let add_out = run_izel_pm_in(&["add", "regex@1.11.0"], &root);
    assert!(
        add_out.status.success(),
        "add command failed: {}",
        String::from_utf8_lossy(&add_out.stderr)
    );

    let add_dev_out = run_izel_pm_in(&["add", "insta@1.39.0", "--dev"], &root);
    assert!(
        add_dev_out.status.success(),
        "add --dev command failed: {}",
        String::from_utf8_lossy(&add_dev_out.stderr)
    );

    let src_after_add = fs::read_to_string(&manifest).expect("manifest should be readable");
    assert!(src_after_add.contains("regex = \"1.11.0\""));
    assert!(src_after_add.contains("[dev-dependencies]"));
    assert!(src_after_add.contains("insta = \"1.39.0\""));

    let remove_out = run_izel_pm_in(&["remove", "regex"], &root);
    assert!(
        remove_out.status.success(),
        "remove command failed: {}",
        String::from_utf8_lossy(&remove_out.stderr)
    );

    let src_after_remove = fs::read_to_string(&manifest).expect("manifest should be readable");
    assert!(!src_after_remove.contains("regex = \"1.11.0\""));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cli_build_in_izel_manifest_project_uses_izelc_path() {
    let root = unique_temp_path("izel-build");
    fs::create_dir_all(root.join("src")).expect("src directory should be created");
    fs::write(
        root.join("Izel.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest should be written");
    fs::write(root.join("src/main.iz"), "forge main() -> i32 { 42 }\n")
        .expect("main source should be written");

    let output = run_izel_pm_in(&["build"], &root);
    assert!(
        output.status.success(),
        "build command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRY-RUN: izelc"));
    assert!(stdout.contains("Build finished."));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cli_run_in_izel_manifest_project_uses_izelc_with_run_flag() {
    let root = unique_temp_path("izel-run");
    fs::create_dir_all(root.join("src")).expect("src directory should be created");
    fs::write(
        root.join("Izel.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest should be written");
    fs::write(root.join("src/main.iz"), "forge main() -> i32 { 42 }\n")
        .expect("main source should be written");

    let output = run_izel_pm_in(&["run"], &root);
    assert!(
        output.status.success(),
        "run command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DRY-RUN: izelc"));
    assert!(stdout.contains("--run"));
    assert!(stdout.contains("Run finished."));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cli_usage_errors_cover_invalid_forms() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["bench", "a", "b"],
        vec!["check", "x"],
        vec!["fmt", "--bad"],
        vec!["lint", "x"],
        vec!["doc", "--bad"],
        vec!["remove"],
        vec!["update", "x"],
        vec!["publish", "x"],
        vec!["clean", "x"],
        vec!["tree", "x"],
        vec!["audit", "x"],
        vec!["build", "--target"],
        vec!["build", "--bad"],
        vec!["test", "--threads"],
        vec!["test", "--threads", "0"],
        vec!["test", "--threads", "abc"],
        vec!["test", "--bogus"],
        vec!["add"],
        vec!["add", "@"],
        vec!["add", "demo", "--bad"],
        vec!["new", "pkg", "--bad"],
        vec!["new", "pkg", "--lib", "--workspace"],
    ];

    for args in cases {
        let output = run_izel_pm(&args);
        assert!(
            !output.status.success(),
            "invalid command {:?} unexpectedly succeeded",
            args
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("usage:")
                || stderr.contains("unknown command")
                || stderr.contains("choose only one")
                || stderr.contains("positive integer")
                || stderr.contains("package spec must"),
            "invalid command {:?} missing usage/error output: {}",
            args,
            stderr
        );
    }
}
