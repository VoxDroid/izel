use std::env;
use std::fs;
use std::io;
use std::path::Path;

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
                fs::write(
                    &main_path,
                    "draw std::io\n\nforge main() !io {\n    std::io::println(\"Hello, Izel!\")\n}\n",
                )?;
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

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_command(&args) {
        Ok(Command::Help) => {
            println!("{}", usage());
        }
        Ok(Command::New { name, kind }) => match create_project(&name, kind) {
            Ok(()) => println!("Created Izel project: {}", name),
            Err(e) => {
                eprintln!("failed to create project {}: {}", name, e);
                std::process::exit(1);
            }
        },
        Ok(Command::Build { release, target }) => {
            println!(
                "Build command accepted. release={}, target={:?}",
                release, target
            );
        }
        Ok(Command::Run { args }) => {
            if args.is_empty() {
                println!("Run command accepted.");
            } else {
                println!("Run command accepted with args: {:?}", args);
            }
        }
        Ok(Command::Test { filter, threads }) => {
            println!(
                "Test command accepted. filter={:?}, threads={:?}",
                filter, threads
            );
        }
        Ok(Command::Bench { filter }) => {
            println!("Bench command accepted. filter={:?}", filter);
        }
        Ok(Command::Check) => {
            println!("Check command accepted.");
        }
        Ok(Command::Fmt { check }) => {
            println!("Fmt command accepted. check={}", check);
        }
        Ok(Command::Lint) => {
            println!("Lint command accepted.");
        }
        Ok(Command::Doc { open }) => {
            println!("Doc command accepted. open={}", open);
        }
        Ok(Command::Add {
            package,
            version,
            dev,
        }) => {
            println!(
                "Add command accepted. package={}, version={:?}, dev={}",
                package, version, dev
            );
        }
        Ok(Command::Remove { package }) => {
            println!("Remove command accepted. package={}", package);
        }
        Ok(Command::Update) => {
            println!("Update command accepted.");
        }
        Ok(Command::Publish) => {
            println!("Publish command accepted.");
        }
        Ok(Command::Clean) => {
            println!("Clean command accepted.");
        }
        Ok(Command::Tree) => {
            println!("Tree command accepted.");
        }
        Ok(Command::Audit) => {
            println!("Audit command accepted.");
        }
        Err(msg) => {
            eprintln!("{}\n{}", msg, usage());
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{create_project, parse_command, usage, Command, NewProjectKind};
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
    fn create_bin_project_writes_manifest_and_main_stub() {
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
        assert!(main_src.contains("draw std::io"));
        assert!(main_src.contains("Hello, Izel!"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_lib_project_writes_library_stub() {
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
