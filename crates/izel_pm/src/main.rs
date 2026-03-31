use std::env;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    New { name: String },
    Build,
    Run { args: Vec<String> },
    Help,
}

fn usage() -> &'static str {
    "izel new <name>\nizel build\nizel run [-- <args>]"
}

fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Ok(Command::Help);
    }

    match args[0].as_str() {
        "new" => {
            if args.len() != 2 {
                return Err("usage: izel new <name>".to_string());
            }
            Ok(Command::New {
                name: args[1].clone(),
            })
        }
        "build" => Ok(Command::Build),
        "run" => {
            let forwarded = if args.len() > 1 && args[1] == "--" {
                args[2..].to_vec()
            } else {
                Vec::new()
            };
            Ok(Command::Run { args: forwarded })
        }
        "--help" | "-h" | "help" => Ok(Command::Help),
        other => Err(format!("unknown command: {}", other)),
    }
}

fn create_project(name: &str) -> io::Result<()> {
    let root = Path::new(name);
    let src = root.join("src");
    fs::create_dir_all(&src)?;

    let manifest_path = root.join("Izel.toml");
    if !manifest_path.exists() {
        fs::write(
            &manifest_path,
            format!(
                "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
                name
            ),
        )?;
    }

    let main_path = src.join("main.iz");
    if !main_path.exists() {
        fs::write(
            &main_path,
            "draw std::io\n\nforge main() !io {\n    std::io::println(\"Hello, Izel!\")\n}\n",
        )?;
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_command(&args) {
        Ok(Command::Help) => {
            println!("{}", usage());
        }
        Ok(Command::New { name }) => match create_project(&name) {
            Ok(()) => println!("Created Izel project: {}", name),
            Err(e) => {
                eprintln!("failed to create project {}: {}", name, e);
                std::process::exit(1);
            }
        },
        Ok(Command::Build) => {
            println!("Build command accepted. Project build wiring will be expanded in follow-up milestones.");
        }
        Ok(Command::Run { args }) => {
            if args.is_empty() {
                println!("Run command accepted.");
            } else {
                println!("Run command accepted with args: {:?}", args);
            }
        }
        Err(msg) => {
            eprintln!("{}\n{}", msg, usage());
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{create_project, parse_command, usage, Command};
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
    fn parse_new_command() {
        let got = parse_command(&["new".into(), "demo".into()]).expect("expected new");
        assert_eq!(
            got,
            Command::New {
                name: "demo".to_string()
            }
        );
    }

    #[test]
    fn parse_build_command() {
        let got = parse_command(&["build".into()]).expect("expected build");
        assert_eq!(got, Command::Build);
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
    fn parse_new_requires_exactly_one_name_argument() {
        let missing = parse_command(&["new".into()]).expect_err("new without name must fail");
        let extra = parse_command(&["new".into(), "demo".into(), "extra".into()])
            .expect_err("new with extra args must fail");

        assert!(missing.contains("usage: izel new <name>"));
        assert!(extra.contains("usage: izel new <name>"));
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
        assert!(text.contains("izel new <name>"));
        assert!(text.contains("izel build"));
        assert!(text.contains("izel run [-- <args>]"));
    }

    #[test]
    fn create_project_writes_manifest_and_main_stub() {
        let root = temp_project_root("create-project");
        let root_str = root.to_string_lossy().to_string();

        create_project(&root_str).expect("project creation should succeed");

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
        create_project(&root_str).expect("project creation should still succeed");

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
        let err = create_project(&root_str).expect_err("manifest write should fail");
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
        let err = create_project(&root_str).expect_err("main write should fail");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        let mut writable = fs::metadata(&src).expect("read src metadata").permissions();
        writable.set_mode(0o755);
        fs::set_permissions(&src, writable).expect("restore src permissions");
        let _ = fs::remove_dir_all(&root);
    }
}
