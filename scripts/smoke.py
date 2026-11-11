#!/usr/bin/env python3
"""Black-box smoke test for a built yadot binary (standard library only)."""
import os
import subprocess
import sys
import tempfile
from pathlib import Path


def run(command, env, cwd=None):
    return subprocess.run(command, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE)


def git(args, env, cwd=None):
    return run(["git", *args], env, cwd)


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def main(binary):
    binary = os.path.abspath(binary)
    with tempfile.TemporaryDirectory(prefix="yadot-smoke-") as directory:
        root = Path(directory)
        home = root / "home"
        home.mkdir()
        yadot = root / "yadot.git"
        remote = root / "remote.git"
        config = root / "gitconfig"
        env = {key: value for key, value in os.environ.items()
               if not key.upper().startswith("GIT_")}
        env.update({
            "HOME": str(home), "YADOT_DIR": str(yadot),
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": str(config),
            "GIT_AUTHOR_NAME": "Smoke Author", "GIT_AUTHOR_EMAIL": "smoke@example.invalid",
            "GIT_COMMITTER_NAME": "Smoke Committer", "GIT_COMMITTER_EMAIL": "smoke@example.invalid",
        })
        check(git(["config", "--file", str(config), "init.defaultBranch", "main"], env).returncode == 0, "configure default branch")
        missing_status = run([binary, "status"], {**env, "YADOT_DIR": str(root / "missing.git")}, home)
        check(missing_status.returncode != 0 and not (root / "missing.git").exists(), "missing status does not create repository")
        initialized = run([binary, "init"], env, home)
        check(initialized.returncode == 0 and yadot.is_dir(), "yadot init")
        check(git(["config", "--get", "core.bare"], env, yadot).stdout.strip() == "true", "bare repository")
        check(git(["config", "--get", "core.worktree"], env, yadot).returncode != 0, "no worktree setting")
        check(git(["init", "--bare", "--initial-branch=main", str(remote)], env).returncode == 0, "init remote")
        check(git(["--git-dir", str(yadot), "remote", "add", "origin", str(remote)], env).returncode == 0, "add remote")

        help_output = run([binary, "--help"], env, home)
        check(help_output.returncode == 0 and "Usage: yadot" in help_output.stdout, "help")
        version = run([binary, "--version"], env, home)
        check(version.returncode == 0 and version.stdout.strip() == "yadot 0.1.0", "version")
        status = run([binary, "status"], env, home)
        check(status.returncode == 0, "initial status")
        subdir = home / "subdir"
        subdir.mkdir()
        tracked = subdir / "tracked.txt"
        tracked.write_text("smoke\n")
        added = run([binary, "add", "tracked.txt"], env, subdir)
        check(added.returncode == 0, "add from subdirectory")
        committed = run([binary, "commit", "-m", "smoke commit"], env, home)
        check(committed.returncode == 0, "commit")
        pushed = run([binary, "push", "origin", "main"], env, home)
        check(pushed.returncode == 0, "push")
        check(git(["--git-dir", str(remote), "show", "main:subdir/tracked.txt"], env).stdout == "smoke\n", "remote content")
        sentinel = yadot / "sentinel"
        sentinel.write_text("keep\n")
        before_ref = git(["--git-dir", str(yadot), "rev-parse", "main"], env).stdout
        repeated = run([binary, "init"], env, home)
        check(repeated.returncode == 0 and sentinel.read_text() == "keep\n", "repeat init preserves repository files")
        check(git(["--git-dir", str(yadot), "rev-parse", "main"], env).stdout == before_ref, "repeat init preserves ref")
        check(git(["--git-dir", str(yadot), "show", "main:subdir/tracked.txt"], env).stdout == "smoke\n", "repeat init preserves content")

        missing = run([binary, "add", "does-not-exist"], env, home)
        direct = git(["--git-dir", str(yadot), "--work-tree", str(home), "add", "--", "does-not-exist"], env, home)
        check(missing.returncode == direct.returncode != 0, "add failure status propagation")
        no_message = run([binary, "commit"], env, home)
        direct_commit = git(["--git-dir", str(yadot), "--work-tree", str(home), "commit"], env, home)
        check(no_message.returncode == direct_commit.returncode != 0, "commit failure status propagation")
        print("smoke: help/version, isolated workflow, subdirectory path, push, and failure propagation passed")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(f"usage: {sys.argv[0]} PATH_TO_YADOT_BINARY")
    try:
        main(sys.argv[1])
    except (AssertionError, OSError, subprocess.SubprocessError) as error:
        print(f"smoke: FAIL: {error}", file=sys.stderr)
        raise SystemExit(1)
