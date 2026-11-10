# Disposable Linux test rig

Like Berger's local test rig, this image tests the checkout without touching
host dotfiles. It contains Rust (including rustfmt and Clippy), Git, Python 3,
and the compiled Yadot binary. It is not a release image.

## Build and run all checks

With Docker installed and its daemon running, from the repository root:

```sh
docker build -f docker/Dockerfile -t yadot-test .
docker run --rm --network none yadot-test
```

The build needs network access to download the base image and tools. If the
Docker bridge cannot create its build veth, use `docker build --network host`
for the build only; tests still run with `--network none`. The test run needs
no network or credentials. It runs formatting, strict Clippy, all Rust tests,
and the Python black-box smoke suite against the release binary. Any failed
check makes the container exit nonzero. Rebuild after code changes; the source
is copied into the image, not mounted from the host.

The smoke suite creates temporary repositories through `yadot init`, a work tree,
and a local bare remote. It exercises initialization, repeat initialization,
status, staging, committing, pushing, and failure propagation. Integration tests
also check init argument rejection, argument bytes, and Unix signals.

## Manual workflow

Start a throwaway shell (the release binary is already on `PATH`):

```sh
docker run -it --rm --network none yadot-test bash
```

Inside the container:

```sh
cd "$HOME"
export GIT_CONFIG_GLOBAL="$HOME/.gitconfig"  # container-local Git configuration
git config --global init.defaultBranch main
yadot init
git init --bare --initial-branch=main /tmp/remote.git
git --git-dir "$HOME/.yadot" remote add origin /tmp/remote.git
printf 'export EDITOR=vi\n' > .bashrc
yadot status --short
yadot add .bashrc
yadot commit -m 'Track shell configuration'
yadot push -u origin main
yadot push
git --git-dir /tmp/remote.git show main:.bashrc

# A Git failure remains nonzero:
yadot add missing-file
printf 'exit status: %s\n' "$?"
```

`HOME` and a synthetic Git identity are configured inside the container; host
Git configuration is not used. The manual example explicitly selects a
container-local Git config file because the test image disables global config.
`yadot init` accepts no extra arguments and follows
Git's native branch and reinitialization behavior. Exiting removes the container
and its test data.
Do not mount your real home, dotfile repository, credentials, or Docker socket
into this rig. No mounts are required.

## Limits

This verifies Linux/Unix behavior, not Windows or other non-Unix runtime paths.
A local bare remote tests Git push semantics without authenticated network
transport. It does not verify GitHub access, SSH keys, HTTPS credentials, or a
live-network push. Those require separate, explicitly configured verification.
