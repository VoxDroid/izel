use std::env;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
enum NewProjectKind {
    Bin,
    Lib,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    New {
        name: String,
        kind: NewProjectKind,
    },
    Build {
        release: bool,
        target: Option<String>,
    },
    Run {
        args: Vec<String>,
    },
    Test {
        filter: Option<String>,
        threads: Option<usize>,
    },
    Bench {
        filter: Option<String>,
    },
    Check,
    Fmt {
        check: bool,
    },
    Lint,
    Doc {
        open: bool,
    },
    Add {
        package: String,
        version: Option<String>,
        dev: bool,
    },
    Remove {
        package: String,
    },
    Update,
    Publish,
    Clean,
    Tree,
    Audit,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectLayout {
    Cargo,
    Izel,
    Unknown,
}

fn usage() -> &'static str {
    "izel new <name> [--lib | --bin | --workspace]\n\
izel build [--release] [--target <triple>]\n\
izel run [-- <args>]\n\
izel test [filter] [--threads <n>]\n\
izel bench [filter]\n\
izel check\n\
izel fmt [--check]\n\
izel lint\n\
izel doc [--open]\n\
izel add <pkg>[@<version>] [--dev]\n\
izel remove <pkg>\n\
izel update\n\
izel publish\n\
izel clean\n\
izel tree\n\
izel audit"
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "new" => parse_new_command(args),
        "build" => parse_build_command(args),
        "run" => {
            let forwarded = if args.len() > 1 && args[1] == "--" {
                args[2..].to_vec()
            } else {
                Vec::new()
            };
            Ok(Command::Run { args: forwarded })
        }
        "test" => parse_test_command(args),
        "bench" => {
            if args.len() > 2 {
                return Err("usage: izel bench [filter]".to_string());
            }
            Ok(Command::Bench {
                filter: args.get(1).cloned(),
            })
        }
        "check" => {
            if args.len() != 1 {
                return Err("usage: izel check".to_string());
            }
            Ok(Command::Check)
        }
        "fmt" => {
            if args.len() == 1 {
                return Ok(Command::Fmt { check: false });
            }
            if args.len() == 2 && args[1] == "--check" {
                return Ok(Command::Fmt { check: true });
            }
            Err("usage: izel fmt [--check]".to_string())
        }
        "lint" => {
            if args.len() != 1 {
                return Err("usage: izel lint".to_string());
            }
            Ok(Command::Lint)
        }
        "doc" => {
            if args.len() == 1 {
                return Ok(Command::Doc { open: false });
            }
            if args.len() == 2 && args[1] == "--open" {
                return Ok(Command::Doc { open: true });
            }
            Err("usage: izel doc [--open]".to_string())
        }
        "add" => parse_add_command(args),
        "remove" => {
            if args.len() != 2 {
                return Err("usage: izel remove <pkg>".to_string());
            }
            Ok(Command::Remove {
                package: args[1].clone(),
            })
        }
        "update" => {
            if args.len() != 1 {
                return Err("usage: izel update".to_string());
            }
            Ok(Command::Update)
        }
        "publish" => {
            if args.len() != 1 {
                return Err("usage: izel publish".to_string());
            }
            Ok(Command::Publish)
        }
        "clean" => {
            if args.len() != 1 {
                return Err("usage: izel clean".to_string());
            }
            Ok(Command::Clean)
        }
        "tree" => {
            if args.len() != 1 {
                return Err("usage: izel tree".to_string());
            }
            Ok(Command::Tree)
        }
        "audit" => {
            if args.len() != 1 {
                return Err("usage: izel audit".to_string());
            }
            Ok(Command::Audit)
        }
        "--help" | "-h" | "help" => Ok(Command::Help),
        other => Err(format!("unknown command: {}", other)),
    }
}

fn parse_new_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("usage: izel new <name> [--lib | --bin | --workspace]".to_string());
    }

    let mut kind = NewProjectKind::Bin;
    let mut kind_flag_count = 0usize;

    for flag in &args[2..] {
        match flag.as_str() {
            "--lib" => {
                kind = NewProjectKind::Lib;
                kind_flag_count += 1;
            }
            "--bin" => {
                kind = NewProjectKind::Bin;
                kind_flag_count += 1;
            }
            "--workspace" => {
                kind = NewProjectKind::Workspace;
                kind_flag_count += 1;
            }
            _ => {
                return Err("usage: izel new <name> [--lib | --bin | --workspace]".to_string());
            }
        }
    }

    if kind_flag_count > 1 {
        return Err("choose only one of --lib, --bin, or --workspace".to_string());
    }

    Ok(Command::New {
        name: args[1].clone(),
        kind,
    })
}

fn parse_build_command(args: &[String]) -> Result<Command, String> {
    let mut release = false;
    let mut target = None;

    let mut idx = 1usize;
    while idx < args.len() {
        match args[idx].as_str() {
            "--release" => {
                release = true;
                idx += 1;
            }
            "--target" => {
                if idx + 1 >= args.len() {
                    return Err("usage: izel build [--release] [--target <triple>]".to_string());
                }
                target = Some(args[idx + 1].clone());
                idx += 2;
            }
            _ => {
                return Err("usage: izel build [--release] [--target <triple>]".to_string());
            }
        }
    }

    Ok(Command::Build { release, target })
}

fn parse_test_command(args: &[String]) -> Result<Command, String> {
    let mut filter = None;
    let mut threads = None;
    let mut idx = 1usize;

    if idx < args.len() && !args[idx].starts_with('-') {
        filter = Some(args[idx].clone());
        idx += 1;
    }

    while idx < args.len() {
        match args[idx].as_str() {
            "--threads" => {
                if idx + 1 >= args.len() {
                    return Err("usage: izel test [filter] [--threads <n>]".to_string());
                }
                let parsed = args[idx + 1]
                    .parse::<usize>()
                    .map_err(|_| "--threads expects a positive integer".to_string())?;
                if parsed == 0 {
                    return Err("--threads expects a positive integer".to_string());
                }
                threads = Some(parsed);
                idx += 2;
            }
            _ => {
                return Err("usage: izel test [filter] [--threads <n>]".to_string());
            }
        }
    }

    Ok(Command::Test { filter, threads })
}

fn parse_add_command(args: &[String]) -> Result<Command, String> {
    if args.len() < 2 {
        return Err("usage: izel add <pkg>[@<version>] [--dev]".to_string());
    }

    let (package, version) = split_package_spec(&args[1])?;
    let mut dev = false;

    for flag in &args[2..] {
        match flag.as_str() {
            "--dev" => dev = true,
            _ => return Err("usage: izel add <pkg>[@<version>] [--dev]".to_string()),
        }
    }

    Ok(Command::Add {
        package,
        version,
        dev,
    })
}

fn split_package_spec(spec: &str) -> Result<(String, Option<String>), String> {
    if let Some((package, version)) = spec.split_once('@') {
        if package.is_empty() || version.is_empty() {
            return Err("package spec must be <pkg> or <pkg>@<version>".to_string());
        }
        Ok((package.to_string(), Some(version.to_string())))
    } else {
        if spec.is_empty() {
            return Err("package spec must be <pkg> or <pkg>@<version>".to_string());
        }
        Ok((spec.to_string(), None))
    }
}

fn create_project(name: &str, kind: NewProjectKind) -> io::Result<()> {
    let root = Path::new(name);
    fs::create_dir_all(root)?;

    let manifest_path = root.join("Izel.toml");
    if !manifest_path.exists() {
        match kind {
            NewProjectKind::Workspace => {
                fs::write(&manifest_path, "[workspace]\nmembers = []\n")?;
            }
            NewProjectKind::Bin | NewProjectKind::Lib => {
                fs::write(
                    &manifest_path,
                    format!(
                        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
                        name
                    ),
                )?;
            }
        }
    }

    match kind {
        NewProjectKind::Workspace => {}
        NewProjectKind::Bin => {
            let src = root.join("src");
            fs::create_dir_all(&src)?;
            let main_path = src.join("main.iz");
            if !main_path.exists() {
                fs::write(&main_path, "forge main() -> i32 {\n    42\n}\n")?;
            }
        }
        NewProjectKind::Lib => {
            let src = root.join("src");
            fs::create_dir_all(&src)?;
            let lib_path = src.join("lib.iz");
            if !lib_path.exists() {
                fs::write(
                    &lib_path,
                    "open forge hello() -> str {\n    \"Hello, Izel!\"\n}\n",
                )?;
            }
        }
    }

    Ok(())
}

fn is_dry_run() -> bool {
    matches!(
        env::var("IZEL_PM_DRY_RUN")
            .ok()
            .map(|v| v.to_ascii_lowercase())
            .as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn run_external(program: &str, args: &[String]) -> Result<(), String> {
    if is_dry_run() {
        println!("DRY-RUN: {} {}", program, args.join(" "));
        return Ok(());
    }

    let status = ProcessCommand::new(program)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {}: {}", program, e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} exited with status {}",
            program,
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string())
        ))
    }
}

fn run_cargo(args: &[String]) -> Result<(), String> {
    run_external("cargo", args)
}

fn detect_project_layout() -> ProjectLayout {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("Cargo.toml").exists() {
        ProjectLayout::Cargo
    } else if cwd.join("Izel.toml").exists() {
        ProjectLayout::Izel
    } else {
        ProjectLayout::Unknown
    }
}

fn parse_bin_path_from_manifest(manifest_src: &str) -> Option<String> {
    let mut in_bin = false;

    for raw_line in manifest_src.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        if line == "[[bin]]" {
            in_bin = true;
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_bin = false;
            continue;
        }

        if in_bin && line.starts_with("path") {
            if let Some((_, rhs)) = line.split_once('=') {
                let value = rhs.trim().trim_matches('"');
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

fn resolve_izel_entry_source() -> Result<PathBuf, String> {
    let cwd = env::current_dir().map_err(|e| format!("failed to read current directory: {}", e))?;
    let manifest_path = cwd.join("Izel.toml");
    let mut candidates = Vec::new();

    if let Ok(manifest_src) = fs::read_to_string(&manifest_path) {
        if let Some(path) = parse_bin_path_from_manifest(&manifest_src) {
            candidates.push(cwd.join(path));
        }
    }

    candidates.push(cwd.join("src/main.iz"));
    candidates.push(cwd.join("main.iz"));
    candidates.push(cwd.join("src/lib.iz"));

    if let Some(path) = candidates.iter().find(|p| p.exists()) {
        return Ok(path.clone());
    }

    let expected = candidates
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Err(format!(
        "could not find an Izel entry source; expected one of: {}",
        expected
    ))
}

fn discover_izelc_candidates() -> Vec<String> {
    let mut out = Vec::new();

    if let Ok(explicit) = env::var("IZEL_PM_IZELC") {
        let explicit = explicit.trim();
        if !explicit.is_empty() {
            out.push(explicit.to_string());
        }
    }

    out.push("izelc".to_string());

    if let Ok(current_exe) = env::current_exe() {
        let sibling = current_exe.with_file_name("izelc");
        if sibling.exists() {
            out.push(sibling.to_string_lossy().to_string());
        }
    }

    out.dedup();
    out
}

fn run_izelc(args: &[String]) -> Result<(), String> {
    if is_dry_run() {
        println!("DRY-RUN: izelc {}", args.join(" "));
        return Ok(());
    }

    for candidate in discover_izelc_candidates() {
        let status = ProcessCommand::new(&candidate)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(status) => {
                if status.success() {
                    return Ok(());
                }

                return Err(format!(
                    "{} exited with status {}",
                    candidate,
                    status
                        .code()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "signal".to_string())
                ));
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                continue;
            }
            Err(err) => {
                return Err(format!("failed to run {}: {}", candidate, err));
            }
        }
    }

    Err(
        "failed to locate izelc executable; install izelc or set IZEL_PM_IZELC=/path/to/izelc"
            .to_string(),
    )
}

fn build_izel_project(
    release: bool,
    target: Option<String>,
    run_after_build: bool,
) -> Result<(), String> {
    let entry = resolve_izel_entry_source()?;
    let mut args = vec![entry.to_string_lossy().to_string()];

    if release {
        args.push("-O".to_string());
        args.push("3".to_string());
    }

    if let Some(t) = target {
        args.push("--target".to_string());
        args.push(t);
    }

    if run_after_build {
        args.push("--run".to_string());
    }

    run_izelc(&args)
}

fn project_root_manifest_path() -> std::path::PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| Path::new(".").to_path_buf())
        .join("Izel.toml")
}

fn upsert_dependency(manifest_src: &str, section: &str, package: &str, value: &str) -> String {
    let mut lines: Vec<String> = manifest_src.lines().map(ToString::to_string).collect();
    let section_header = format!("[{}]", section);

    let mut sec_start = None;
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() == section_header {
            sec_start = Some(idx);
            break;
        }
    }

    if sec_start.is_none() {
        if !lines.is_empty() && !lines.last().is_some_and(|l| l.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(section_header);
        lines.push(format!("{} = {}", package, value));
        return format!("{}\n", lines.join("\n"));
    }

    let start = sec_start.expect("section start must exist");
    let mut end = lines.len();
    for (idx, line) in lines.iter().enumerate().skip(start + 1) {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            end = idx;
            break;
        }
    }

    let key_prefix = package;
    for line in lines.iter_mut().take(end).skip(start + 1) {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key_prefix)
            && trimmed[key_prefix.len()..].trim_start().starts_with('=')
        {
            *line = format!("{} = {}", package, value);
            return format!("{}\n", lines.join("\n"));
        }
    }

    lines.insert(end, format!("{} = {}", package, value));
    format!("{}\n", lines.join("\n"))
}

fn remove_dependency_from_section(manifest_src: &str, section: &str, package: &str) -> String {
    let lines: Vec<String> = manifest_src.lines().map(ToString::to_string).collect();
    let mut out = Vec::with_capacity(lines.len());
    let section_header = format!("[{}]", section);

    let mut in_section = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == section_header;
            out.push(line);
            continue;
        }

        if in_section {
            let left = line.split('=').next().map(str::trim).unwrap_or_default();
            if left == package {
                continue;
            }
        }
        out.push(line);
    }

    format!("{}\n", out.join("\n"))
}

fn add_dependency_to_manifest(
    package: &str,
    version: Option<&str>,
    dev: bool,
) -> Result<(), String> {
    let manifest_path = project_root_manifest_path();
    let src = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {}", manifest_path.display(), e))?;
    let section = if dev {
        "dev-dependencies"
    } else {
        "dependencies"
    };
    let value = format!("\"{}\"", version.unwrap_or("*"));
    let updated = upsert_dependency(&src, section, package, &value);
    fs::write(&manifest_path, updated)
        .map_err(|e| format!("failed to write {}: {}", manifest_path.display(), e))?;
    println!(
        "Updated {}: added {} to [{}]",
        manifest_path.display(),
        package,
        section
    );
    Ok(())
}

fn remove_dependency_from_manifest(package: &str) -> Result<(), String> {
    let manifest_path = project_root_manifest_path();
    let src = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read {}: {}", manifest_path.display(), e))?;
    let without_deps = remove_dependency_from_section(&src, "dependencies", package);
    let updated = remove_dependency_from_section(&without_deps, "dev-dependencies", package);
    fs::write(&manifest_path, updated)
        .map_err(|e| format!("failed to write {}: {}", manifest_path.display(), e))?;
    println!("Updated {}: removed {}", manifest_path.display(), package);
    Ok(())
}

fn open_docs_if_requested(open: bool) -> Result<(), String> {
    if !open {
        return Ok(());
    }

    if is_dry_run() {
        println!("DRY-RUN: opening target/doc/index.html");
        return Ok(());
    }

    let index = Path::new("target/doc/index.html");
    if !index.exists() {
        return Err("documentation index not found at target/doc/index.html".to_string());
    }

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = ProcessCommand::new("open");
        c.arg(index);
        c
    };

    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = ProcessCommand::new("xdg-open");
        c.arg(index);
        c
    };

    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = ProcessCommand::new("cmd");
        c.args(["/C", "start", "", &index.to_string_lossy()]);
        c
    };

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    cmd.spawn()
        .map_err(|e| format!("failed to open docs in browser: {}", e))?;
    Ok(())
}

fn execute_command(command: Command) -> Result<(), String> {
    match command {
        Command::Help => {
            println!("{}", usage());
            Ok(())
        }
        Command::New { name, kind } => {
            create_project(&name, kind)
                .map_err(|e| format!("failed to create project {}: {}", name, e))?;
            println!("Created Izel project: {}", name);
            Ok(())
        }
        Command::Build { release, target } => {
            match detect_project_layout() {
                ProjectLayout::Cargo => {
                    let mut args = vec!["build".to_string()];
                    if release {
                        args.push("--release".to_string());
                    }
                    if let Some(t) = target {
                        args.push("--target".to_string());
                        args.push(t);
                    }
                    run_cargo(&args)?;
                }
                ProjectLayout::Izel => {
                    build_izel_project(release, target, false)?;
                }
                ProjectLayout::Unknown => {
                    return Err(
                        "no project manifest found (expected Cargo.toml or Izel.toml)".to_string(),
                    );
                }
            }
            println!("Build finished.");
            Ok(())
        }
        Command::Run { args } => {
            match detect_project_layout() {
                ProjectLayout::Cargo => {
                    let mut cargo_args = vec!["run".to_string()];
                    if !args.is_empty() {
                        cargo_args.push("--".to_string());
                        cargo_args.extend(args);
                    }
                    run_cargo(&cargo_args)?;
                }
                ProjectLayout::Izel => {
                    if !args.is_empty() {
                        return Err(
                            "izel run does not yet support forwarded runtime args for standalone Izel manifests"
                                .to_string(),
                        );
                    }
                    build_izel_project(false, None, true)?;
                }
                ProjectLayout::Unknown => {
                    return Err(
                        "no project manifest found (expected Cargo.toml or Izel.toml)".to_string(),
                    );
                }
            }
            println!("Run finished.");
            Ok(())
        }
        Command::Test { filter, threads } => {
            let mut args = vec!["test".to_string()];
            if let Some(f) = filter {
                args.push(f);
            }
            if let Some(n) = threads {
                args.push("--".to_string());
                args.push(format!("--test-threads={}", n));
            }
            run_cargo(&args)?;
            println!("Tests finished.");
            Ok(())
        }
        Command::Bench { filter } => {
            let mut args = vec!["bench".to_string()];
            if let Some(f) = filter {
                args.push(f);
            }
            run_cargo(&args)?;
            println!("Bench run finished.");
            Ok(())
        }
        Command::Check => {
            run_cargo(&["check".to_string()])?;
            println!("Check finished.");
            Ok(())
        }
        Command::Fmt { check } => {
            let mut args = vec!["fmt".to_string()];
            if check {
                args.push("--".to_string());
                args.push("--check".to_string());
            }
            run_cargo(&args)?;
            println!("Format task finished.");
            Ok(())
        }
        Command::Lint => {
            let workspace_args = vec![
                "clippy".to_string(),
                "--workspace".to_string(),
                "--all-targets".to_string(),
                "--".to_string(),
                "-D".to_string(),
                "warnings".to_string(),
            ];
            if run_cargo(&workspace_args).is_err() {
                run_cargo(&[
                    "clippy".to_string(),
                    "--".to_string(),
                    "-D".to_string(),
                    "warnings".to_string(),
                ])?;
            }
            println!("Lint finished.");
            Ok(())
        }
        Command::Doc { open } => {
            run_cargo(&["doc".to_string(), "--no-deps".to_string()])?;
            open_docs_if_requested(open)?;
            println!("Documentation generated.");
            Ok(())
        }
        Command::Add {
            package,
            version,
            dev,
        } => add_dependency_to_manifest(&package, version.as_deref(), dev),
        Command::Remove { package } => remove_dependency_from_manifest(&package),
        Command::Update => {
            run_cargo(&["update".to_string()])?;
            println!("Update finished.");
            Ok(())
        }
        Command::Publish => {
            run_cargo(&["publish".to_string(), "--dry-run".to_string()])?;
            println!("Publish dry-run finished.");
            Ok(())
        }
        Command::Clean => {
            run_cargo(&["clean".to_string()])?;
            println!("Clean finished.");
            Ok(())
        }
        Command::Tree => {
            run_cargo(&["tree".to_string()])?;
            println!("Dependency tree finished.");
            Ok(())
        }
        Command::Audit => {
            if run_external("cargo", &["audit".to_string()]).is_err() {
                return Err("cargo-audit is not installed. Install with `cargo install cargo-audit --locked`".to_string());
            }
            println!("Audit finished.");
            Ok(())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_command(&args).and_then(execute_command) {
        Ok(()) => {}
        Err(msg) => {
            eprintln!("{}\n{}", msg, usage());
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        create_project, parse_command, split_package_spec, usage, Command, NewProjectKind,
    };
    use std::fs;
    use std::io;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_project_root(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after UNIX epoch")
            .as_nanos();
        path.push(format!("izel-pm-test-{}-{}", name, nonce));
        path
    }

    #[test]
    fn parse_help_when_no_args() {
        let got = parse_command(&[]).expect("expected help");
        assert_eq!(got, Command::Help);
    }

    #[test]
    fn parse_new_command_defaults_to_bin() {
        let got = parse_command(&["new".into(), "demo".into()]).expect("expected new");
        assert_eq!(
            got,
            Command::New {
                name: "demo".to_string(),
                kind: NewProjectKind::Bin,
            }
        );
    }

    #[test]
    fn parse_new_command_accepts_workspace_kind() {
        let got = parse_command(&["new".into(), "demo".into(), "--workspace".into()])
            .expect("expected new workspace");
        assert_eq!(
            got,
            Command::New {
                name: "demo".to_string(),
                kind: NewProjectKind::Workspace,
            }
        );
    }

    #[test]
    fn parse_build_command() {
        let got = parse_command(&["build".into()]).expect("expected build");
        assert_eq!(
            got,
            Command::Build {
                release: false,
                target: None,
            }
        );
    }

    #[test]
    fn parse_build_command_with_release_and_target() {
        let got = parse_command(&[
            "build".into(),
            "--release".into(),
            "--target".into(),
            "wasm32-unknown-unknown".into(),
        ])
        .expect("expected build with options");
        assert_eq!(
            got,
            Command::Build {
                release: true,
                target: Some("wasm32-unknown-unknown".to_string()),
            }
        );
    }

    #[test]
    fn parse_run_with_forwarded_args() {
        let got = parse_command(&["run".into(), "--".into(), "a".into(), "b".into()])
            .expect("expected run");
        assert_eq!(
            got,
            Command::Run {
                args: vec!["a".to_string(), "b".to_string()]
            }
        );
    }

    #[test]
    fn parse_run_without_separator_drops_extra_args() {
        let got = parse_command(&["run".into(), "a".into(), "b".into()]).expect("run parse");
        assert_eq!(got, Command::Run { args: vec![] });
    }

    #[test]
    fn parse_test_with_filter_and_threads() {
        let got = parse_command(&[
            "test".into(),
            "typeck".into(),
            "--threads".into(),
            "4".into(),
        ])
        .expect("test parse");
        assert_eq!(
            got,
            Command::Test {
                filter: Some("typeck".to_string()),
                threads: Some(4),
            }
        );
    }

    #[test]
    fn parse_add_with_version_and_dev() {
        let got = parse_command(&["add".into(), "izel-http@2.1".into(), "--dev".into()])
            .expect("add parse");
        assert_eq!(
            got,
            Command::Add {
                package: "izel-http".to_string(),
                version: Some("2.1".to_string()),
                dev: true,
            }
        );
    }

    #[test]
    fn parse_misc_subcommands_and_options() {
        assert_eq!(
            parse_command(&["bench".into(), "core".into()]).unwrap(),
            Command::Bench {
                filter: Some("core".to_string())
            }
        );
        assert_eq!(parse_command(&["check".into()]).unwrap(), Command::Check);
        assert_eq!(
            parse_command(&["fmt".into()]).unwrap(),
            Command::Fmt { check: false }
        );
        assert_eq!(
            parse_command(&["fmt".into(), "--check".into()]).unwrap(),
            Command::Fmt { check: true }
        );
        assert_eq!(parse_command(&["lint".into()]).unwrap(), Command::Lint);
        assert_eq!(
            parse_command(&["doc".into()]).unwrap(),
            Command::Doc { open: false }
        );
        assert_eq!(
            parse_command(&["doc".into(), "--open".into()]).unwrap(),
            Command::Doc { open: true }
        );
        assert_eq!(
            parse_command(&["remove".into(), "pkg".into()]).unwrap(),
            Command::Remove {
                package: "pkg".to_string()
            }
        );
        assert_eq!(parse_command(&["update".into()]).unwrap(), Command::Update);
        assert_eq!(
            parse_command(&["publish".into()]).unwrap(),
            Command::Publish
        );
        assert_eq!(parse_command(&["clean".into()]).unwrap(), Command::Clean);
        assert_eq!(parse_command(&["tree".into()]).unwrap(), Command::Tree);
        assert_eq!(parse_command(&["audit".into()]).unwrap(), Command::Audit);
    }

    #[test]
    fn parse_new_rejects_invalid_flags_and_conflicts() {
        let bad = parse_command(&["new".into(), "demo".into(), "--bad".into()])
            .expect_err("invalid new flag must fail");
        assert!(bad.contains("usage: izel new <name>"));

        let conflict = parse_command(&[
            "new".into(),
            "demo".into(),
            "--lib".into(),
            "--workspace".into(),
        ])
        .expect_err("conflicting kind flags must fail");
        assert!(conflict.contains("choose only one"));
    }

    #[test]
    fn parse_build_rejects_invalid_forms() {
        let missing_target = parse_command(&["build".into(), "--target".into()])
            .expect_err("missing target triple must fail");
        assert!(missing_target.contains("usage: izel build"));

        let bad_flag = parse_command(&["build".into(), "--bogus".into()])
            .expect_err("unknown build flag must fail");
        assert!(bad_flag.contains("usage: izel build"));
    }

    #[test]
    fn parse_test_rejects_invalid_thread_inputs() {
        let missing = parse_command(&["test".into(), "--threads".into()])
            .expect_err("missing threads value must fail");
        assert!(missing.contains("usage: izel test"));

        let zero = parse_command(&["test".into(), "--threads".into(), "0".into()])
            .expect_err("zero threads must fail");
        assert!(zero.contains("positive integer"));

        let non_numeric = parse_command(&["test".into(), "--threads".into(), "abc".into()])
            .expect_err("non-numeric threads must fail");
        assert!(non_numeric.contains("positive integer"));

        let unknown = parse_command(&["test".into(), "--bogus".into()])
            .expect_err("unknown test arg must fail");
        assert!(unknown.contains("usage: izel test"));
    }

    #[test]
    fn parse_add_and_split_package_spec_cover_error_paths() {
        let missing_add = parse_command(&["add".into()]).expect_err("add without pkg must fail");
        assert!(missing_add.contains("usage: izel add"));

        let bad_add_flag = parse_command(&["add".into(), "demo".into(), "--bad".into()])
            .expect_err("unknown add flag must fail");
        assert!(bad_add_flag.contains("usage: izel add"));

        assert!(split_package_spec("@").is_err());
        assert!(split_package_spec("pkg@").is_err());
        assert!(split_package_spec("").is_err());
        assert_eq!(
            split_package_spec("pkg").unwrap(),
            ("pkg".to_string(), None)
        );
        assert_eq!(
            split_package_spec("pkg@1.0.0").unwrap(),
            ("pkg".to_string(), Some("1.0.0".to_string()))
        );
    }

    #[test]
    fn parse_misc_subcommands_reject_extra_arguments() {
        assert!(parse_command(&["bench".into(), "a".into(), "b".into()]).is_err());
        assert!(parse_command(&["check".into(), "x".into()]).is_err());
        assert!(parse_command(&["fmt".into(), "--bad".into()]).is_err());
        assert!(parse_command(&["lint".into(), "x".into()]).is_err());
        assert!(parse_command(&["doc".into(), "--bad".into()]).is_err());
        assert!(parse_command(&["remove".into()]).is_err());
        assert!(parse_command(&["update".into(), "x".into()]).is_err());
        assert!(parse_command(&["publish".into(), "x".into()]).is_err());
        assert!(parse_command(&["clean".into(), "x".into()]).is_err());
        assert!(parse_command(&["tree".into(), "x".into()]).is_err());
        assert!(parse_command(&["audit".into(), "x".into()]).is_err());
    }

    #[test]
    fn parse_new_requires_name_argument() {
        let missing = parse_command(&["new".into()]).expect_err("new without name must fail");
        assert!(missing.contains("usage: izel new <name>"));
    }

    #[test]
    fn parse_unknown_command_returns_error() {
        let err = parse_command(&["deploy".into()]).expect_err("unknown command should fail");
        assert!(err.contains("unknown command: deploy"));
    }

    #[test]
    fn parse_help_aliases_are_accepted() {
        assert_eq!(parse_command(&["help".into()]).unwrap(), Command::Help);
        assert_eq!(parse_command(&["--help".into()]).unwrap(), Command::Help);
        assert_eq!(parse_command(&["-h".into()]).unwrap(), Command::Help);
    }

    #[test]
    fn usage_text_lists_supported_commands() {
        let text = usage();
        assert!(text.contains("izel new <name> [--lib | --bin | --workspace]"));
        assert!(text.contains("izel build [--release] [--target <triple>]"));
        assert!(text.contains("izel run [-- <args>]"));
        assert!(text.contains("izel test [filter] [--threads <n>]"));
        assert!(text.contains("izel bench [filter]"));
        assert!(text.contains("izel add <pkg>[@<version>] [--dev]"));
    }

    #[test]
    fn create_bin_project_writes_manifest_and_main_file() {
        let root = temp_project_root("create-project");
        let root_str = root.to_string_lossy().to_string();

        create_project(&root_str, NewProjectKind::Bin).expect("project creation should succeed");

        let manifest = root.join("Izel.toml");
        let main = root.join("src/main.iz");
        assert!(manifest.exists());
        assert!(main.exists());

        let manifest_src = fs::read_to_string(&manifest).expect("manifest should be readable");
        let main_src = fs::read_to_string(&main).expect("main should be readable");

        assert!(manifest_src.contains("[package]"));
        assert!(manifest_src.contains("[dependencies]"));
        assert!(main_src.contains("forge main() -> i32"));
        assert!(main_src.contains("42"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_lib_project_writes_library_file() {
        let root = temp_project_root("create-lib");
        let root_str = root.to_string_lossy().to_string();

        create_project(&root_str, NewProjectKind::Lib).expect("lib project creation should work");

        let manifest = root.join("Izel.toml");
        let lib = root.join("src/lib.iz");
        assert!(manifest.exists());
        assert!(lib.exists());

        let lib_src = fs::read_to_string(&lib).expect("lib should be readable");
        assert!(lib_src.contains("open forge hello()"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_workspace_project_writes_workspace_manifest_only() {
        let root = temp_project_root("create-workspace");
        let root_str = root.to_string_lossy().to_string();

        create_project(&root_str, NewProjectKind::Workspace)
            .expect("workspace project creation should work");

        let manifest = root.join("Izel.toml");
        let src_dir = root.join("src");
        assert!(manifest.exists());
        assert!(!src_dir.exists());

        let manifest_src = fs::read_to_string(&manifest).expect("manifest should be readable");
        assert!(manifest_src.contains("[workspace]"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_project_is_idempotent_for_existing_files() {
        let root = temp_project_root("idempotent");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("setup src dir");

        let manifest = root.join("Izel.toml");
        let main = src.join("main.iz");
        fs::write(&manifest, "[package]\nname=\"keep\"\nversion=\"0.1.0\"\n")
            .expect("write existing manifest");
        fs::write(&main, "forge main() { give }\n").expect("write existing main");

        let root_str = root.to_string_lossy().to_string();
        create_project(&root_str, NewProjectKind::Bin)
            .expect("project creation should still succeed");

        let manifest_after = fs::read_to_string(&manifest).expect("manifest should still exist");
        let main_after = fs::read_to_string(&main).expect("main should still exist");

        assert!(manifest_after.contains("name=\"keep\""));
        assert!(main_after.contains("forge main()"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_lib_project_is_idempotent_for_existing_files() {
        let root = temp_project_root("idempotent-lib");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("setup src dir");

        let manifest = root.join("Izel.toml");
        let lib = src.join("lib.iz");
        fs::write(&manifest, "[package]\nname=\"keep\"\nversion=\"0.1.0\"\n")
            .expect("write existing manifest");
        fs::write(&lib, "open forge existing() -> str {\n    \"keep\"\n}\n")
            .expect("write existing lib");

        let root_str = root.to_string_lossy().to_string();
        create_project(&root_str, NewProjectKind::Lib)
            .expect("project creation should still succeed");

        let manifest_after = fs::read_to_string(&manifest).expect("manifest should still exist");
        let lib_after = fs::read_to_string(&lib).expect("lib should still exist");

        assert!(manifest_after.contains("name=\"keep\""));
        assert!(lib_after.contains("open forge existing()"));

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn create_project_surfaces_manifest_write_error_for_read_only_root() {
        let root = temp_project_root("manifest-write-error");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("create source directory");

        let mut root_perms = fs::metadata(&root)
            .expect("read root metadata")
            .permissions();
        root_perms.set_mode(0o555);
        fs::set_permissions(&root, root_perms).expect("set root read-only");

        let root_str = root.to_string_lossy().to_string();
        let err =
            create_project(&root_str, NewProjectKind::Bin).expect_err("manifest write should fail");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        let mut writable = fs::metadata(&root)
            .expect("read root metadata")
            .permissions();
        writable.set_mode(0o755);
        fs::set_permissions(&root, writable).expect("restore root permissions");
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn create_project_surfaces_main_write_error_for_read_only_src() {
        let root = temp_project_root("main-write-error");
        let src = root.join("src");
        fs::create_dir_all(&src).expect("create src directory");
        fs::write(
            root.join("Izel.toml"),
            "[package]\nname = \"keep\"\nversion = \"0.1.0\"\n",
        )
        .expect("write existing manifest");

        let mut src_perms = fs::metadata(&src).expect("read src metadata").permissions();
        src_perms.set_mode(0o555);
        fs::set_permissions(&src, src_perms).expect("set src read-only");

        let root_str = root.to_string_lossy().to_string();
        let err =
            create_project(&root_str, NewProjectKind::Bin).expect_err("main write should fail");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        let mut writable = fs::metadata(&src).expect("read src metadata").permissions();
        writable.set_mode(0o755);
        fs::set_permissions(&src, writable).expect("restore src permissions");
        let _ = fs::remove_dir_all(&root);
    }
}
