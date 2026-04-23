use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve repository root")
}

fn temp_iz_file(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "izel-driver-runtime-io-{}-{}-{}.iz",
        std::process::id(),
        nonce,
        serial
    ));
    fs::write(&path, content).expect("failed to write runtime io fixture");
    path
}

fn temp_data_file_path(prefix: &str, ext: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "izel-driver-runtime-data-{}-{}-{}-{}.{}",
        prefix,
        std::process::id(),
        nonce,
        serial,
        ext
    ));
    path
}

fn temp_data_dir_path(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    let serial = TEMP_NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.push(format!(
        "izel-driver-runtime-dir-{}-{}-{}-{}",
        prefix,
        std::process::id(),
        nonce,
        serial
    ));
    path
}

fn run_izelc_from_repo(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izelc"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to execute izelc")
}

fn run_izelc_with_stdin_from_repo(args: &[&str], stdin_input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_izelc"))
        .args(args)
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to execute izelc with piped stdin");

    {
        let mut stdin = child
            .stdin
            .take()
            .expect("expected child stdin pipe to be present");
        stdin
            .write_all(stdin_input.as_bytes())
            .expect("failed to write stdin payload");
    }

    child
        .wait_with_output()
        .expect("failed to collect izelc output")
}

fn extract_runtime_stdout(full_stdout: &str) -> String {
    let mut in_runtime = false;
    let mut between_markers = Vec::new();
    let mut after_footer = Vec::new();
    let mut seen_footer = false;

    for line in full_stdout.lines() {
        if line == "--- JIT Execution ---" {
            in_runtime = true;
            continue;
        }

        if !in_runtime && !seen_footer {
            continue;
        }

        if line.starts_with("JIT Exit Code:") {
            continue;
        }

        if line == "----------------------" {
            seen_footer = true;
            in_runtime = false;
            continue;
        }

        if in_runtime {
            between_markers.push(line.to_string());
        } else if seen_footer {
            after_footer.push(line.to_string());
        }
    }

    let lines = if between_markers.is_empty() {
        after_footer
    } else {
        between_markers
    };

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[test]
fn runtime_io_streams_snapshot_stdout_and_stderr_separately() {
    let source = r#"draw std/io

forge main() -> int {
    println("stdout-line")
    eprintln("stderr-line")
    println_int(7)
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    let stdout_snapshot = "stdout-line\n7\n";
    let stderr_snapshot = "stderr-line\n";

    assert_eq!(runtime_stdout, stdout_snapshot);
    assert_eq!(stderr, stderr_snapshot);
}

#[test]
fn runtime_io_streams_preserve_escaped_string_snapshots() {
    let source = r#"draw std/io

forge main() -> int {
    println("stdout-\x41\tend")
    eprintln("stderr-\u{1F600}")
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    let stdout_snapshot = "stdout-A\tend\n";
    let stderr_snapshot = "stderr-😀\n";

    assert_eq!(runtime_stdout, stdout_snapshot);
    assert_eq!(stderr, stderr_snapshot);
}

#[test]
fn runtime_io_file_roundtrip_snapshot() {
    let data_path = temp_data_file_path("roundtrip", "txt");
    let escaped_path = data_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let path = "{path}"
    write_file(path, "alpha-line")
    let loaded = read_file(path)
    println(loaded)
    free_str(loaded)
    give 0
}}
"#,
        path = escaped_path
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    assert_eq!(runtime_stdout, "alpha-line\n");
    assert_eq!(stderr, "");

    let written = fs::read_to_string(&data_path).expect("runtime should create output file");
    assert_eq!(written, "alpha-line");

    let _ = fs::remove_file(&data_path);
}

#[test]
fn runtime_io_file_ops_append_exists_remove_and_list_snapshot() {
    let data_dir = temp_data_dir_path("ops");
    fs::create_dir(&data_dir).expect("failed to create runtime data directory");
    let data_path = data_dir.join("ledger.txt");

    let escaped_path = data_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_dir = data_dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let path = "{path}"
    let dir = "{dir}"
    write_file(path, "line-a")
    append_file(path, "\nline-b")
    println_int(file_exists(path))
    let listed = list_dir(dir)
    print(listed)
    free_str(listed)
    println_int(remove_file(path))
    println_int(file_exists(path))
    give 0
}}
"#,
        path = escaped_path,
        dir = escaped_dir
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    assert_eq!(runtime_stdout, "1\nledger.txt\n0\n0\n");
    assert_eq!(stderr, "");
    assert!(
        !data_path.exists(),
        "remove_file should delete the target file"
    );

    let _ = fs::remove_file(&data_path);
    let _ = fs::remove_dir(&data_dir);
}

#[test]
fn runtime_io_missing_paths_surface_error_status() {
    let missing_file = temp_data_file_path("missing", "txt");
    let missing_dir = temp_data_dir_path("missing");
    let escaped_file = missing_file
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_dir = missing_dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let path = "{path}"
    let dir = "{dir}"

    let loaded = read_file(path)
    free_str(loaded)
    println_int(io_last_status())

    let listed = list_dir(dir)
    free_str(listed)
    println_int(io_last_status())

    let removed = remove_file(path)
    println_int(removed)
    println_int(io_last_status())
    give 0
}}
"#,
        path = escaped_file,
        dir = escaped_dir
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    let values: Vec<i32> = runtime_stdout
        .lines()
        .map(|line| line.parse::<i32>().expect("runtime line should be an i32"))
        .collect();
    assert_eq!(
        values.len(),
        4,
        "expected four status lines, got: {runtime_stdout}"
    );
    assert_ne!(
        values[0], 0,
        "read_file missing-path status should be nonzero"
    );
    assert_ne!(
        values[1], 0,
        "list_dir missing-path status should be nonzero"
    );
    assert_eq!(
        values[2], -1,
        "remove_file missing-path status should return -1"
    );
    assert_ne!(
        values[3], 0,
        "remove_file missing-path io_last_status should be nonzero"
    );
    assert_eq!(stderr, "");
}

#[test]
fn runtime_io_bool_exists_and_status_helpers_work() {
    let data_dir = temp_data_dir_path("try");
    fs::create_dir(&data_dir).expect("failed to create runtime data directory");
    let data_path = data_dir.join("try.txt");

    let escaped_path = data_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_dir = data_dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let path = "{path}"
    let dir = "{dir}"
    write_file(path, "alpha")
    append_file(path, "\nbeta")
    let listed = list_dir(dir)
    print(listed)
    free_str(listed)
    println_int(io_last_status())

    given file_exists_bool(path) {{
        println("exists")
    }}

    remove_file(path)
    give 0
}}
"#,
        path = escaped_path,
        dir = escaped_dir
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "try.txt\n0\nexists\n");
    assert_eq!(stderr, "");

    let _ = fs::remove_file(&data_path);
    let _ = fs::remove_dir(&data_dir);
}

#[test]
fn runtime_io_stdin_numeric_parsing_snapshots() {
    let int_source = r#"draw std/io

forge main() -> int {
    let value = read_stdin_int()
    println_int(value)
    give 0
}
"#;

    let int_input = temp_iz_file(int_source);
    let int_input_arg = int_input.to_string_lossy().to_string();
    let int_output = run_izelc_with_stdin_from_repo(&["--run", &int_input_arg], "37\n");
    let _ = fs::remove_file(&int_input);

    assert!(
        int_output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&int_output.stdout),
        String::from_utf8_lossy(&int_output.stderr)
    );

    let int_stdout = String::from_utf8_lossy(&int_output.stdout);
    let int_stderr = String::from_utf8_lossy(&int_output.stderr);
    assert_eq!(extract_runtime_stdout(&int_stdout), "37\n");
    assert_eq!(int_stderr, "");

    let float_source = r#"draw std/io

forge main() -> int {
    read_stdin_float()
    println_int(io_last_status())
    give 0
}
"#;

    let float_input = temp_iz_file(float_source);
    let float_input_arg = float_input.to_string_lossy().to_string();
    let float_output = run_izelc_with_stdin_from_repo(&["--run", &float_input_arg], "3.5\n");
    let _ = fs::remove_file(&float_input);

    assert!(
        float_output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&float_output.stdout),
        String::from_utf8_lossy(&float_output.stderr)
    );

    let float_stdout = String::from_utf8_lossy(&float_output.stdout);
    let float_stderr = String::from_utf8_lossy(&float_output.stderr);
    assert_eq!(extract_runtime_stdout(&float_stdout), "0\n");
    assert_eq!(float_stderr, "");

    let invalid_int_source = r#"draw std/io

forge main() -> int {
    let value = read_stdin_int()
    println_int(value)
    println_int(io_last_status())
    give 0
}
"#;

    let invalid_int_input = temp_iz_file(invalid_int_source);
    let invalid_int_arg = invalid_int_input.to_string_lossy().to_string();
    let invalid_int_output =
        run_izelc_with_stdin_from_repo(&["--run", &invalid_int_arg], "not-a-number\n");
    let _ = fs::remove_file(&invalid_int_input);

    assert!(
        invalid_int_output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&invalid_int_output.stdout),
        String::from_utf8_lossy(&invalid_int_output.stderr)
    );

    let invalid_int_stdout = String::from_utf8_lossy(&invalid_int_output.stdout);
    let invalid_int_stderr = String::from_utf8_lossy(&invalid_int_output.stderr);
    assert_eq!(extract_runtime_stdout(&invalid_int_stdout), "0\n-2\n");
    assert_eq!(invalid_int_stderr, "");

    let invalid_float_source = r#"draw std/io

forge main() -> int {
    read_stdin_float()
    println("float-read")
    println_int(io_last_status())
    give 0
}
"#;

    let invalid_float_input = temp_iz_file(invalid_float_source);
    let invalid_float_arg = invalid_float_input.to_string_lossy().to_string();
    let invalid_float_output =
        run_izelc_with_stdin_from_repo(&["--run", &invalid_float_arg], "oops\n");
    let _ = fs::remove_file(&invalid_float_input);

    assert!(
        invalid_float_output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&invalid_float_output.stdout),
        String::from_utf8_lossy(&invalid_float_output.stderr)
    );

    let invalid_float_stdout = String::from_utf8_lossy(&invalid_float_output.stdout);
    let invalid_float_stderr = String::from_utf8_lossy(&invalid_float_output.stderr);
    assert_eq!(
        extract_runtime_stdout(&invalid_float_stdout),
        "float-read\n-2\n"
    );
    assert_eq!(invalid_float_stderr, "");
}

#[test]
fn runtime_io_try_helpers_and_error_kind_snapshot() {
    let data_path = temp_data_file_path("try-kind", "txt");
    let missing_path = temp_data_file_path("try-kind-missing", "txt");

    let escaped_data = data_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_missing = missing_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let missing = "{missing}"
    let missing_loaded = try_read_file(missing)
    free_str(missing_loaded)
    println_int(io_last_error_kind())
    println(io_last_error_kind_name())

    let path = "{path}"
    try_write_file(path, "alpha")
    println_int(io_last_status())
    let loaded = try_read_file(path)
    println(loaded)
    free_str(loaded)
    remove_file(path)
    give 0
}}
"#,
        missing = escaped_missing,
        path = escaped_data,
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "1\nnot_found\n0\nalpha\n");
    assert_eq!(stderr, "");

    let _ = fs::remove_file(&data_path);
}

#[test]
fn runtime_io_structured_listing_and_bytes_hex_snapshot() {
    let data_dir = temp_data_dir_path("structured");
    fs::create_dir(&data_dir).expect("failed to create runtime data directory");
    let text_path = data_dir.join("note.txt");
    let bytes_path = data_dir.join("payload.bin");

    let escaped_dir = data_dir
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_text = text_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let escaped_bytes = bytes_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let dir = "{dir}"
    let text_path = "{text_path}"
    let bytes_path = "{bytes_path}"

    write_file(text_path, "hello")
    write_file_bytes_hex(bytes_path, "0001feff")
    println_int(io_last_status())

    let structured = list_dir_structured(dir)
    print(structured)
    free_str(structured)

    let loaded_hex = read_file_bytes_hex(bytes_path)
    println(loaded_hex)
    free_str(loaded_hex)

    remove_file(text_path)
    remove_file(bytes_path)
    give 0
}}
"#,
        dir = escaped_dir,
        text_path = escaped_text,
        bytes_path = escaped_bytes,
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        extract_runtime_stdout(&stdout),
        "0\nnote.txt\tfile\npayload.bin\tfile\n0001feff\n"
    );
    assert_eq!(stderr, "");

    let _ = fs::remove_file(&text_path);
    let _ = fs::remove_file(&bytes_path);
    let _ = fs::remove_dir(&data_dir);
}

#[test]
fn runtime_io_large_append_and_special_path_stress() {
    let data_dir = temp_data_dir_path("stress path");
    fs::create_dir(&data_dir).expect("failed to create runtime data directory");
    let data_path = data_dir.join("payload spaced #1.txt");

    let escaped_path = data_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let path = "{path}"
    write_file(path, "")

    ~i = 0
    while i < 256 {{
        append_file(path, "0123456789abcdef")
        i = i + 1
    }}

    println_int(io_last_status())
    let loaded = read_file(path)
    free_str(loaded)
    println_int(io_last_status())
    give 0
}}
"#,
        path = escaped_path,
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "0\n0\n");
    assert_eq!(stderr, "");

    let written = fs::read(&data_path).expect("expected stress payload to exist");
    assert_eq!(written.len(), 256 * 16, "unexpected stress payload size");

    let _ = fs::remove_file(&data_path);
    let _ = fs::remove_dir(&data_dir);
}

#[test]
fn runtime_io_cross_platform_path_separator_snapshot() {
    let data_dir = temp_data_dir_path("path-style");
    fs::create_dir(&data_dir).expect("failed to create runtime data directory");
    let data_path = data_dir.join("cross-platform.txt");

    let native = data_path.to_string_lossy().to_string();
    let forward = native.replace('\\', "/");
    let backward = native.replace('/', "\\\\");

    let escaped_native = native.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_forward = forward.replace('\\', "\\\\").replace('"', "\\\"");
    let escaped_backward = backward.replace('\\', "\\\\").replace('"', "\\\"");

    let source = format!(
        r#"draw std/io

forge main() -> int {{
    let native = "{native}"
    let forward = "{forward}"
    let backward = "{backward}"

    write_file(native, "x")
    println_int(file_exists(native))
    println_int(file_exists(forward))
    println_int(file_exists(backward))
    remove_file(native)
    give 0
}}
"#,
        native = escaped_native,
        forward = escaped_forward,
        backward = escaped_backward,
    );

    let input = temp_iz_file(&source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let values: Vec<i32> = extract_runtime_stdout(&stdout)
        .lines()
        .map(|line| line.parse::<i32>().expect("runtime line should be an i32"))
        .collect();

    assert_eq!(values.len(), 3, "expected three path existence lines");
    assert_eq!(values[0], 1, "native path should always resolve");

    if cfg!(windows) {
        assert_eq!(values[1], 1, "forward-slash path should resolve on windows");
        assert_eq!(values[2], 1, "backslash path should resolve on windows");
    } else {
        assert_eq!(values[1], 1, "forward-slash path should resolve on unix");
        assert_eq!(values[2], 0, "backslash path should not resolve on unix");
    }

    assert_eq!(stderr, "");

    let _ = fs::remove_file(&data_path);
    let _ = fs::remove_dir(&data_dir);
}

#[test]
fn runtime_io_empty_stdin_numeric_reports_parse_kind() {
    let source = r#"draw std/io

forge main() -> int {
    let int_value = read_stdin_int()
    println_int(int_value)
    println_int(io_last_error_kind())

    read_stdin_float()
    println_int(io_last_error_kind())
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_with_stdin_from_repo(&["--run", &input_arg], "\n\n");
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "0\n5\n5\n");
    assert_eq!(stderr, "");
}

#[test]
fn runtime_control_flow_while_plain_assignment_executes_iterations() {
    let source = r#"draw std/io

forge main() -> int {
    ~i = 0
    while i < 3 {
        println_int(i)
        i = i + 1
    }
    println("done")
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "0\n1\n2\ndone\n");
    assert_eq!(stderr, "");
}

#[test]
fn runtime_control_flow_while_tilde_reassignment_executes_iterations() {
    let source = r#"draw std/io

forge main() -> int {
    ~i = 0
    while i < 3 {
        println_int(i)
        ~i = i + 1
    }
    println("done")
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(extract_runtime_stdout(&stdout), "0\n1\n2\ndone\n");
    assert_eq!(stderr, "");
}

#[test]
fn runtime_control_flow_each_placeholder_callee_reports_error_without_segfault() {
    let source = r#"draw std/io

forge main() -> int {
    each x in [1, 2, 3] {
        println_int(x)
    }
    println("each-done")
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();
    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        !output.status.success(),
        "expected compilation to fail cleanly"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unresolved callee")
            || stderr.contains("placeholder from earlier lowering"),
        "missing unresolved-callee diagnostic\nstderr:\n{}",
        stderr
    );
}
