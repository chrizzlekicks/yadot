use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return ExitCode::from(2);
    };

    let command = command.to_string_lossy();
    if command == "help" || command == "--help" || command == "-h" {
        print_help();
        return ExitCode::SUCCESS;
    }
    if command == "--version" || command == "-V" {
        println!("yadot 0.1.0");
        return ExitCode::SUCCESS;
    }

    if !matches!(
        command.as_ref(),
        "init" | "status" | "add" | "commit" | "push"
    ) {
        eprintln!("yadot: unknown command `{command}`");
        print_help();
        return ExitCode::from(2);
    }
    if command == "init" && args.next().is_some() {
        eprintln!("yadot: init does not accept arguments");
        return ExitCode::from(2);
    }

    let home = match env::var_os("HOME") {
        Some(home) if !home.is_empty() => PathBuf::from(home),
        Some(_) => {
            eprintln!("yadot: HOME is empty");
            return ExitCode::from(1);
        }
        None => {
            eprintln!("yadot: HOME is not set");
            return ExitCode::from(1);
        }
    };
    let git_dir = match env::var_os("YADOT_DIR") {
        Some(git_dir) if !git_dir.is_empty() => PathBuf::from(git_dir),
        Some(_) => {
            eprintln!("yadot: YADOT_DIR is empty");
            return ExitCode::from(1);
        }
        None => home.join(".yadot"),
    };

    let mut git_args: Vec<OsString> = vec!["--git-dir".into(), git_dir.into_os_string()];
    if command == "init" {
        git_args.extend(["init".into(), "--bare".into()]);
    } else {
        git_args.extend([
            "--work-tree".into(),
            home.into_os_string(),
            command.as_ref().into(),
        ]);
        git_args.extend(args);
        if command == "add" {
            if git_args.len() == 5 {
                eprintln!("yadot: add requires at least one path");
                return ExitCode::from(2);
            }
            git_args.insert(5, OsString::from("--"));
        }
    }

    #[cfg(unix)]
    {
        let error = Command::new("git").args(&git_args).exec();
        eprintln!("yadot: could not run git: {error}");
        ExitCode::from(1)
    }

    #[cfg(not(unix))]
    {
        match Command::new("git").args(&git_args).status() {
            Ok(status) => match status.code() {
                Some(code) => std::process::exit(code),
                None => {
                    eprintln!("yadot: git terminated by signal");
                    ExitCode::from(1)
                }
            },
            Err(error) => {
                eprintln!("yadot: could not run git: {error}");
                ExitCode::from(1)
            }
        }
    }
}

fn print_help() {
    println!("Usage: yadot <init|status|add|commit|push> [arguments]");
    println!();
    println!("Uses YADOT_DIR as the bare repository (default: $HOME/.yadot)");
}
