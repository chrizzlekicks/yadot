use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "yadot-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn run(home: Option<&OsStr>, yadot_dir: Option<&OsStr>, args: &[&OsStr]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_yadot"));
    command.args(args);
    match home {
        Some(value) => {
            command.env("HOME", value);
        }
        None => {
            command.env_remove("HOME");
        }
    }
    match yadot_dir {
        Some(value) => {
            command.env("YADOT_DIR", value);
        }
        None => {
            command.env_remove("YADOT_DIR");
        }
    }
    command.output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn help_and_version() {
    let root = temp_dir("basic");
    let help = run(Some(root.as_os_str()), None, &[OsStr::new("--help")]);
    assert_eq!(help.status.code(), Some(0));
    assert!(text(&help.stdout).contains("Usage: yadot"));
    let version = run(Some(root.as_os_str()), None, &[OsStr::new("--version")]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(text(&version.stdout).trim(), "yadot 0.1.0");
}

#[test]
fn missing_and_unknown_command() {
    let root = temp_dir("commands");
    let missing = run(Some(root.as_os_str()), None, &[]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(text(&missing.stdout).contains("Usage: yadot"));
    let unknown = run(Some(root.as_os_str()), None, &[OsStr::new("wat")]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(text(&unknown.stderr).contains("unknown command `wat`"));
}

#[test]
fn missing_and_empty_environment_is_rejected() {
    let root = temp_dir("environment");
    let missing_home = run(None, None, &[OsStr::new("status")]);
    assert_eq!(missing_home.status.code(), Some(1));
    assert!(text(&missing_home.stderr).contains("HOME is not set"));
    let empty_home = run(Some(OsStr::new("")), None, &[OsStr::new("status")]);
    assert_eq!(empty_home.status.code(), Some(1));
    assert!(text(&empty_home.stderr).contains("HOME is empty"));
    let empty_dir = run(
        Some(root.as_os_str()),
        Some(OsStr::new("")),
        &[OsStr::new("status")],
    );
    assert_eq!(empty_dir.status.code(), Some(1));
    assert!(text(&empty_dir.stderr).contains("YADOT_DIR is empty"));
}

#[test]
fn add_requires_paths() {
    let root = temp_dir("add-missing");
    let output = run(Some(root.as_os_str()), None, &[OsStr::new("add")]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("add requires at least one path"));
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::ExitStatusExt;
    use std::path::Path;

    fn fake_git(root: &Path) -> (PathBuf, PathBuf) {
        let bin = root.join("git");
        let log = root.join("args");
        fs::write(&bin, b"#!/bin/sh\nif [ \"${FAKE_GIT_SIGNAL:-0}\" = 1 ]; then kill -TERM $$; fi\nprintf '%s\\0' \"$@\" > \"$FAKE_GIT_LOG\"\nprintf '%s' \"$FAKE_GIT_STDOUT\"\nprintf '%s' \"$FAKE_GIT_STDERR\" >&2\nexit \"${FAKE_GIT_STATUS:-0}\"\n").unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        (bin, log)
    }

    fn run_fake(root: &Path, args: &[OsString], status: &str) -> Output {
        let (_bin, log) = fake_git(root);
        let mut command = Command::new(env!("CARGO_BIN_EXE_yadot"));
        command
            .args(args)
            .env("HOME", root)
            .env("YADOT_DIR", root.join("repo"))
            .env("PATH", root)
            .env("FAKE_GIT_LOG", &log)
            .env("FAKE_GIT_STDOUT", "fake stdout\n")
            .env("FAKE_GIT_STDERR", "fake stderr\n")
            .env("FAKE_GIT_STATUS", status)
            .env("FAKE_GIT_SIGNAL", "0");
        command.output().unwrap()
    }

    fn run_fake_signal(root: &Path, args: &[OsString]) -> Output {
        let (_bin, log) = fake_git(root);
        Command::new(env!("CARGO_BIN_EXE_yadot"))
            .args(args)
            .env("HOME", root)
            .env("YADOT_DIR", root.join("repo"))
            .env("PATH", root)
            .env("FAKE_GIT_LOG", log)
            .env("FAKE_GIT_SIGNAL", "1")
            .output()
            .unwrap()
    }

    fn logged(path: &Path) -> Vec<Vec<u8>> {
        fs::read(path)
            .unwrap()
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(Vec::from)
            .collect()
    }

    #[test]
    fn init_uses_bare_git_arguments_only() {
        let root = temp_dir("init-args");
        let output = run_fake(&root, &[OsString::from("init")], "0");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, b"fake stdout\n");
        assert_eq!(output.stderr, b"fake stderr\n");
        assert_eq!(
            logged(&root.join("args")),
            vec![
                b"--git-dir".to_vec(),
                root.join("repo").into_os_string().into_vec(),
                b"init".to_vec(),
                b"--bare".to_vec(),
            ]
        );
    }

    #[test]
    fn init_preserves_non_utf8_repository_path() {
        let root = temp_dir("init-nonutf8");
        let (_bin, log) = fake_git(&root);
        let bad = OsString::from_vec(vec![b'r', 0x80, b'e', b'p', b'o']);
        let output = Command::new(env!("CARGO_BIN_EXE_yadot"))
            .arg("init")
            .env("HOME", &root)
            .env("YADOT_DIR", &bad)
            .env("PATH", &root)
            .env("FAKE_GIT_LOG", &log)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(logged(&log)[1], bad.into_vec());
    }

    #[test]
    fn init_rejects_arguments_without_launching_git() {
        for extra in ["other-directory", "--bare", "--initial-branch=main", "--"] {
            let root = temp_dir("init-reject");
            let (_bin, log) = fake_git(&root);
            let output = Command::new(env!("CARGO_BIN_EXE_yadot"))
                .arg("init")
                .arg(extra)
                .env("HOME", &root)
                .env("YADOT_DIR", root.join("repo"))
                .env("PATH", &root)
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(text(&output.stderr).contains("init does not accept arguments"));
            assert!(!log.exists());
        }
    }

    #[test]
    fn forwards_git_arguments_output_status_and_paths() {
        let root = temp_dir("forward");
        let args = vec![
            OsString::from("add"),
            OsString::from("-dash"),
            OsString::from("space path"),
        ];
        let output = run_fake(&root, &args, "7");
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"fake stdout\n");
        assert_eq!(output.stderr, b"fake stderr\n");
        assert_eq!(
            logged(&root.join("args")),
            vec![
                b"--git-dir".to_vec(),
                root.join("repo").into_os_string().into_vec(),
                b"--work-tree".to_vec(),
                root.clone().into_os_string().into_vec(),
                b"add".to_vec(),
                b"--".to_vec(),
                b"-dash".to_vec(),
                b"space path".to_vec(),
            ]
        );
        for command_args in [
            vec![
                OsString::from("commit"),
                OsString::from("-m"),
                OsString::from("a message"),
            ],
            vec![
                OsString::from("push"),
                OsString::from("origin"),
                OsString::from("main"),
            ],
        ] {
            let output = run_fake(&root, &command_args, "0");
            assert_eq!(output.status.code(), Some(0));
            let recorded = logged(&root.join("args"));
            assert_eq!(
                &recorded[4..],
                &command_args
                    .iter()
                    .map(|arg| arg.clone().into_vec())
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn preserves_git_signal_status() {
        let root = temp_dir("signal");
        let output = run_fake_signal(&root, &[OsString::from("status")]);
        assert_eq!(output.status.signal(), Some(15));
    }

    #[test]
    fn forwards_non_utf8_argument() {
        let root = temp_dir("nonutf8");
        let bad = OsString::from_vec(vec![b'x', 0x80, b'y']);
        let output = run_fake(&root, &[OsString::from("status"), bad.clone()], "0");
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(logged(&root.join("args")).last().unwrap(), &bad.into_vec());
    }

    #[test]
    fn reports_git_launch_failure() {
        let root = temp_dir("launch-failure");
        let output = Command::new(env!("CARGO_BIN_EXE_yadot"))
            .arg("status")
            .env("HOME", &root)
            .env("PATH", root.join("missing"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(text(&output.stderr).contains("could not run git"));
    }
}

#[cfg(windows)]
#[test]
fn forwards_windows_git_exit_status() {
    let root = temp_dir("windows-status");
    let source = root.join("git.rs");
    fs::write(&source, "fn main() { std::process::exit(std::env::var(\"FAKE_GIT_STATUS\").unwrap().parse().unwrap()); }").unwrap();
    assert!(Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(root.join("git.exe"))
        .status()
        .unwrap()
        .success());
    for code in [7, 256] {
        let output = Command::new(env!("CARGO_BIN_EXE_yadot"))
            .arg("status")
            .env("HOME", &root)
            .env("YADOT_DIR", root.join("repo"))
            .env("PATH", &root)
            .env("FAKE_GIT_STATUS", code.to_string())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
    }
}
