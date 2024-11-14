# Tasks

Development tasks for rustic.

You can run this file with [mask](https://github.com/jacobdeichert/mask/).

Install `mask` with `cargo install mask`.

## check

> Checks the library for syntax and HIR errors.

Bash:

```bash
cargo check --no-default-features \
    && cargo check --all-features
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "check --no-default-features").WaitForExit()
[Diagnostics.Process]::Start("cargo", "cargo check --all-features").WaitForExit()
```

## ci

> Continually runs the development routines.

Bash:

```bash
mask loop dev
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "loop dev").WaitForExit()
```

## clean

> Removes all build artifacts.

Bash:

```bash
cargo clean
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "clean").WaitForExit()
```

## dev

> Runs the development routines

Bash:

```bash
$MASK format \
    && $MASK lint \
    && $MASK test \
    && $MASK doc
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "format").WaitForExit()
[Diagnostics.Process]::Start("mask", "lint").WaitForExit()
[Diagnostics.Process]::Start("mask", "test").WaitForExit()
[Diagnostics.Process]::Start("mask", "doc").WaitForExit()
```

## doc (crate)

> Opens the crate documentation

Bash:

```bash
cargo doc --all-features --no-deps --open $crate
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "doc --all-features --no-deps --open $crate").WaitForExit()
```

## format

> Run formatters on the repository.

### format cargo

> Runs the formatter on all Rust files.

Bash:

```bash
cargo fmt --all
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "fmt --all").WaitForExit()
```

### format dprint

> Runs the formatter on md, json, and toml files

Bash:

```bash
dprint fmt
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("dprint", "fmt").WaitForExit()
```

### format all

> Runs all the formatters.

Bash:

```bash
$MASK format cargo \
    && $MASK format dprint
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "format cargo").WaitForExit()
[Diagnostics.Process]::Start("mask", "format dprint").WaitForExit()
```

## inverse-deps (crate)

> Lists all crates that depend on the given crate

Bash:

```bash
cargo tree -e features -i $crate
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "tree -e features -i $crate").WaitForExit()
```

## lint

> Runs the linter

Bash:

```bash
$MASK check \
    && cargo clippy --no-default-features -- -D warnings \
    && cargo clippy --all-features -- -D warnings
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "check").WaitForExit()
[Diagnostics.Process]::Start("cargo", "clippy --no-default-features -- -D warnings").WaitForExit()
[Diagnostics.Process]::Start("cargo", "clippy --all-features -- -D warnings").WaitForExit()
```

## loop (action)

> Continually runs some recipe from this file.

Bash:

```bash
watchexec -w src -- "$MASK $action"
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("watchexec", "-w src -- $MASK $action).WaitForExit()
```

## miri (tests)

> Looks for undefined behavior in the (non-doc) test suite.

**NOTE**: This requires the nightly toolchain.

Bash:

```bash
cargo +nightly miri test --all-features -q --lib --tests $tests
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "+nightly miri test --all-features -q --lib --tests $tests").WaitForExit()
```

## nextest

> Runs the whole test suite with nextest.

### nextest ignored

> Runs the whole test suite with nextest on the workspace, including ignored
> tests.

Bash:

```bash
cargo nextest run -r --all-features --workspace -- --ignored
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "nextest run -r --all-features --workspace -- --ignored").WaitForExit()
```

### nextest ws

> Runs the whole test suite with nextest on the workspace.

Bash:

```bash
cargo nextest run -r --all-features --workspace
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "nextest run -r --all-features --workspace").WaitForExit()
```

### nextest test

> Runs a single test with nextest.

- test
  - flags: -t, --test
  - type: string
  - desc: Only run the specified test target
  - required

Bash:

```bash
cargo nextest run -r --all-features -E "test($test)"
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "nextest run -r --all-features -E 'test($test)'").WaitForExit()
```

## pr

> Prepare a Contribution/Pull request and run necessary checks and lints

Bash:

```bash
$MASK fmt \
    && $MASK test \
    && $MASK lint
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "fmt").WaitForExit()
[Diagnostics.Process]::Start("mask", "test").WaitForExit()
[Diagnostics.Process]::Start("mask", "lint").WaitForExit()
```

## test

> Runs the test suites.

Bash:

```bash
$MASK check \
    && $MASK lint
    && cargo test --all-features
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("mask", "check").WaitForExit()
[Diagnostics.Process]::Start("mask", "lint").WaitForExit()
[Diagnostics.Process]::Start("cargo", "test --all-features").WaitForExit()
```

## test-restic

> Run a restic test against the server

Bash:

```bash
export RESTIC_REPOSITORY=rest:http://127.0.0.1:8000/ci_repo
export RESTIC_PASSWORD=restic
export RESTIC_REST_USERNAME=restic
export RESTIC_REST_PASSWORD=restic
restic init
restic backup tests/fixtures/test_data/test_repo_source
restic backup tests/fixtures/test_data/test_repo_source
restic check
restic forget --keep-last 1 --prune
```

PowerShell:

```powershell
$env:RESTIC_REPOSITORY = "rest:http://127.0.0.1:8000/ci_repo";
$env:RESTIC_PASSWORD = "restic";
$env:RESTIC_REST_USERNAME = "restic";
$env:RESTIC_REST_PASSWORD = "restic";
restic init
restic backup tests/fixtures/test_data/test_repo_source
restic backup tests/fixtures/test_data/test_repo_source
restic check
restic forget --keep-last 1 --prune
```

## test-server

> Run our server for testing

Bash:

```bash
cargo run -- serve -c tests/fixtures/test_data/rustic_server.toml -v
```

PowerShell:

```powershell
[Diagnostics.Process]::Start("cargo", "run -- serve -c tests/fixtures/test_data/rustic_server.toml -v").WaitForExit()

```
<!-- cargo run -- serve -c tests/fixtures/test_data/rustic_server.toml -v -->

## test-restic-server

> Run a restic server for testing

Bash:

```bash
tests/fixtures/rest_server/rest-server.exe --path ./tests/generated/test_storage/ --htpasswd-file ./tests/fixtures/test_data/.htpasswd --log ./tests/fixtures/rest_server/response2.log
```

PowerShell:

```powershell
[Diagnostics.Process]::Start(".\\tests\\fixtures\\rest_server\\rest-server.exe", "--path .\\tests\\generated\\test_storage\\ --htpasswd-file .\\tests\\fixtures\\test_data\\.htpasswd --log .\\tests\\fixtures\\rest_server\\response2.log").WaitForExit()
```

## loop-test-server

> Run our server for testing in a loop

PowerShell:

```powershell
watchexec --stop-signal "CTRL+C" -r -w src -w tests -- "cargo run -- serve -c tests/fixtures/test_data/rustic_server.toml -v"
```

## hurl

> Run a hurl test against the server

Bash:

```bash
hurl -i tests/fixtures/hurl/endpoints.hurl
```

PowerShell:

```powershell
hurl -i tests/fixtures/hurl/endpoints.hurl
```

## debug-test (test)

> Run a single test with debug output

- test
  - flags: -t, --test
  - type: string
  - desc: Only run the specified test target
  - required

Bash:

```bash
$env:RUST_LOG="debug"; cargo test --package rustic_server --lib -- $test --exact --nocapture --show-output
```

PowerShell:

```powershell
$env:RUST_LOG="debug"; cargo test --package rustic_server --lib -- $test --exact --nocapture --show-output
```
