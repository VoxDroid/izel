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
    use super::{parse_command, Command};

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
}
