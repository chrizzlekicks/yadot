# yadot

Yadot is a small wrapper around Git's bare-repository dotfile workflow. Version 0.1 forwards selected operations to Git; it does not manage a separate dotfile format.

## Installation

Install Rust and Cargo using [rustup](https://rustup.rs/), then install yadot from this checkout:

```sh
cargo install --path .
```

Git must also be installed and available on `PATH`. Yadot invokes the `git` executable directly.

## Prerequisites

Yadot requires a bare Git repository. Use an existing bare repository, or initialize one:

```sh
git init --bare --initial-branch=main "$HOME/.yadot"
```

The default bare-repository path is `$HOME/.yadot`. Set `YADOT_DIR` to another path:

```sh
YADOT_DIR=/path/to/dotfiles.git yadot status
```

`YADOT_DIR` may be relative; Git resolves that path from the current working directory. `HOME` is the work tree. Paths passed to `yadot add` are therefore interpreted relative to the directory where yadot is run, just as they are by Git; for example, from `$HOME/.config`, `yadot add git/config` tracks `$HOME/.config/git/config`.

## Commands

```sh
yadot status
yadot add .bashrc .config/git/config
yadot commit -m "Track shell configuration"
yadot push -u origin main
```

`status`, `commit`, and `push` forward their options and other arguments to Git. For example, `yadot status --short` and `yadot commit -a -m "Update config"` are passed through unchanged.

`add` accepts one or more paths, and yadot places Git's `--` before them. This means names beginning with a dash are paths, not options:

```sh
yadot add -notes
```

Do not pass add options; `yadot add` is for paths only. At least one path is required.

## Remote and first push

Configure a remote with raw Git, using the same bare repository:

```sh
git --git-dir "$HOME/.yadot" remote add origin <repository-url>
yadot push -u origin main
```

Replace `main` with the branch you committed. The `-u` sets the upstream for the first push; later `yadot push` can use that upstream. A remote is not created automatically.

## Git identity

Git needs an author and committer identity for commits. Configure one before running `yadot commit`:

```sh
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

## Exit status and limitations

For a normal Git subprocess exit, yadot returns Git's exit status and leaves Git's output and errors intact. On Unix, yadot replaces itself with Git, so Git's signals, output, and exit status are preserved. If Git cannot be started, yadot reports an error and exits with status 1. On non-Unix platforms, signal termination is reported as status 1 because the standard library fallback cannot reproduce a signal-derived status.

Usage errors (for example, an unknown command or `yadot add` without a path) exit 2. Missing or empty `HOME`/`YADOT_DIR` and other launch failures exit 1.

Yadot 0.1 supports only `status`, `add`, `commit`, and `push`. It does not clone repositories, apply or install dotfiles, manage symlinks or backups, or provide setup automation. To show only tracked changes in status, optionally configure Git directly:

```sh
git config --global status.showUntrackedFiles no
```

## Development and testing

From the repository root:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build
cargo build --release
python3 scripts/smoke.py target/debug/yadot
```

The smoke harness uses temporary repositories and requires Python 3 and Git.

For a disposable Linux environment with all tools included, see the
[Docker test rig](docker/README.md). It supports automated checks and a manual
bare-repository workflow without mounting your host home or credentials.
