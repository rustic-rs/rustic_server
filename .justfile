# 'Just' Configuration
# Loads .env file for variables to be used in
# in this just file

set dotenv-load := true

# Ignore recipes that are commented out

set ignore-comments := true

# Set shell for Windows OSs:
# If you have PowerShell Core installed and want to use it,
# use `pwsh.exe` instead of `powershell.exe`

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Set shell for non-Windows OSs:

set shell := ["bash", "-uc"]

export RUST_BACKTRACE := "1"
export RUST_LOG := ""
export CI := "1"

build:
    cargo build --all-features
    cargo build -r --all-features

b: build

check:
    cargo check --no-default-features
    cargo check --all-features

c: check

ci:
    just loop . dev

dev: format lint test

d: dev

format-dprint:
    dprint fmt

format-cargo:
    cargo fmt --all

format: format-cargo format-dprint

fmt: format

rev:
    cargo insta review

inverse-deps crate:
    cargo tree -e features -i {{ crate }}

lint: check
    cargo clippy --no-default-features -- -D warnings
    cargo clippy --all-targets --all-features -- -D warnings

loop dir action:
    watchexec -w {{ dir }} -- "just {{ action }}"

test: check lint
    cargo test --all-targets --all-features --workspace

test-ignored: check lint
    cargo test --all-targets --all-features --workspace -- --ignored

t: test test-ignored

test-restic $RESTIC_REPOSITORY="rest:http://restic:restic@127.0.0.1:8080/ci_repo" $RESTIC_PASSWORD="restic":
    restic init
    restic backup tests/fixtures/test_data/test_repo_source
    restic backup src
    restic check
    restic forget --keep-last 1 --prune
    restic snapshots

test-server:
    cargo run -- serve -c tests/fixtures/test_data/rustic_server.toml -v

test-restic-server:
    tests/fixtures/rest_server/rest-server.exe --path ./tests/generated/test_storage/ --htpasswd-file ./tests/fixtures/test_data/.htpasswd --log ./tests/fixtures/rest_server/response2.log

loop-test-server:
    watchexec --stop-signal "CTRL+C" -r -w src -w tests -- "cargo run -- serve -c tests/fixtures/test_data/rustic_server.toml -v"

hurl:
    hurl -i tests/fixtures/hurl/endpoints.hurl

dbg-test test_name $RUST_LOG="debug":
    cargo test --package rustic_server --lib -- {{ test_name }} --exact --nocapture --show-output

build-docker version="0.4.0":
    podman build containers --build-arg RUSTIC_SERVER_VERSION=v{{ version }} --format docker --tag rustic_server:v{{ version }}