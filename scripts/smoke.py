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
        env = os.environ.copy()
        env.update({
            "HOME": str(home), "YADOT_DIR": str(yadot),
            "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": str(config),
            "GIT_AUTHOR_NAME": "Smoke Author", "GIT_AUTHOR_EMAIL": "smoke@example.invalid",
            "GIT_COMMITTER_NAME": "Smoke Committer", "GIT_COMMITTER_EMAIL": "smoke@example.invalid",
        })
        check(git(["init", "--bare", "--initial-branch=main", str(yadot)], env).returncode == 0, "init yadot")
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
