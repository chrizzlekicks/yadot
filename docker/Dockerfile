# Local test rig, not a release image. Build from the repository root.
FROM rust:1-slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    git python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add rustfmt clippy

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
COPY scripts/smoke.py ./scripts/smoke.py

# Keep test identity and configuration inside the disposable container.
ENV HOME=/tmp/yadot-home \
    GIT_CONFIG_NOSYSTEM=1 \
    GIT_CONFIG_GLOBAL=/dev/null \
    GIT_AUTHOR_NAME="Yadot Test" \
    GIT_AUTHOR_EMAIL=test@example.invalid \
    GIT_COMMITTER_NAME="Yadot Test" \
    GIT_COMMITTER_EMAIL=test@example.invalid
RUN mkdir -p "$HOME" && cargo build --release --locked
ENV PATH="/src/target/release:${PATH}"

CMD ["sh", "-c", "cargo fmt --check && cargo clippy --locked --all-targets --all-features -- -D warnings && cargo test --locked --all-targets --all-features && python3 scripts/smoke.py target/release/yadot && python3 tests/smoke_env.py target/release/yadot"]
