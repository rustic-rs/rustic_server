//! Acceptance test: runs the application as a subprocess and asserts its
//! output for given argument combinations matches what is expected.
//!
//! Modify and/or delete these as you see fit to test the specific needs of
//! your application.
//!
//! For more information, see:
//! <https://docs.rs/abscissa_core/latest/abscissa_core/testing/index.html>

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

use std::{net::SocketAddr, path::PathBuf};

use crate::_impl::AssertCmdExt;
use anyhow::{Ok, Result};
use assert_cmd::Command;
use rstest::{fixture, rstest};
use rustic_server::config::RusticServerConfig;
use serial_test::file_serial;

#[fixture]
fn setup() -> Result<Command> {
    let runner = Command::cargo_bin(env!("CARGO_PKG_NAME").replace("_", "-"))?;

    Ok(runner)
}

/// Use command-line argument value
#[rstest]
#[file_serial]
#[ignore = "FIXME: This test doesn't run in CI because it needs to bind to a port."]
fn test_serve_with_args_passes(setup: Result<Command>) -> Result<()> {
    let assert = setup?
        .arg("serve")
        .args(["--listen", "127.0.0.1:8001"])
        .args(["--htpasswd-file", "tests/fixtures/test_data/.htpasswd"])
        .args([
            "--tls",
            "--tls-cert",
            "tests/fixtures/test_data/certs/test.crt",
            "--tls-key",
            "tests/fixtures/test_data/certs/test.key",
        ])
        .args([
            "--private-repos",
            "--acl-path",
            "tests/fixtures/test_data/acl.toml",
        ])
        .args(["--path", "tests/generated/test_storage"])
        .args(["--max-size", "1000"])
        .test_mode_args()
        .assert();

    assert
        .stdout(predicates::str::contains("Parsed socket address."))
        .stdout(predicates::str::contains("ACL is enabled."))
        .stdout(predicates::str::contains(
            "Authentication is enabled by default.",
        ))
        .stdout(predicates::str::contains("TLS is enabled."))
        .stdout(predicates::str::contains(
            "Listening on: `https://127.0.0.1:8001`",
        ))
        .stdout(predicates::str::contains("Shutting down gracefully ..."))
        .success();

    Ok(())
}

/// Use configured value
#[rstest]
#[file_serial]
#[ignore = "FIXME: This test doesn't run in CI because it needs to bind to a port."]
fn start_with_config_no_args(setup: Result<Command>) -> Result<()> {
    let mut config = RusticServerConfig::default();
    config.server.listen = Some(SocketAddr::from(([127, 0, 0, 1], 8081)));
    config.storage.quota = Some(1000);
    config.acl.acl_path = Some(PathBuf::from("tests/fixtures/test_data/acl.toml"));
    config.auth.htpasswd_file = Some(PathBuf::from("tests/fixtures/test_data/.htpasswd"));

    let assert = setup?
        .test_mode_args()
        .config(&config)
        .arg("serve")
        .assert();

    assert
        .stdout(predicates::str::contains("Using configuration file:"))
        .stdout(predicates::str::contains("ACL is enabled."))
        .stdout(predicates::str::contains("TLS is disabled."))
        .stdout(predicates::str::contains("Starting web server ..."))
        .stdout(predicates::str::contains(
            "Listening on: `http://127.0.0.1:8081`",
        ))
        .stdout(predicates::str::contains("Shutting down gracefully ..."))
        .success();

    Ok(())
}

/// Check merge precedence
#[rstest]
#[file_serial]
#[ignore = "FIXME: This test doesn't run in CI because it needs to bind to a port."]
fn start_with_config_and_args(setup: Result<Command>) -> Result<()> {
    let mut config = RusticServerConfig::default();
    config.server.listen = Some(SocketAddr::from(([127, 0, 0, 1], 8081)));
    config.acl.acl_path = Some(PathBuf::from("tests/fixtures/test_data/acl.toml"));

    let assert = setup?
        .test_mode_args()
        .config(&config)
        .arg("serve")
        .args(["--listen", "127.0.0.1:8001"])
        .args(["--htpasswd-file", "tests/fixtures/test_data/.htpasswd"])
        .assert();

    assert
        .stdout(predicates::str::contains("ACL is enabled."))
        .stdout(predicates::str::contains(
            "Authentication is enabled by default.",
        ))
        .stdout(predicates::str::contains("TLS is disabled."))
        .stdout(predicates::str::contains(
            "Listening on: `http://127.0.0.1:8001",
        ))
        .stdout(predicates::str::contains("Shutting down gracefully ..."))
        .success();

    Ok(())
}

// /// Override configured value with command-line argument
// #[test]
// fn start_with_config_and_args() {
//     let mut config = RusticServerConfig::default();
//     config.hello.recipient = "configured recipient".to_owned();

//     let mut runner = RUNNER.clone();
//     let mut cmd = runner
//         .config(&config)
//         .args(&["start", "acceptance", "test"])
//         .capture_stdout()
//         .run();

//     cmd.stdout().expect_line("Hello, acceptance test!");
//     cmd.wait().unwrap().expect_success();
// }

#[rstest]
fn test_version_no_args_passes(setup: Result<Command>) -> Result<()> {
    let assert = setup?.arg("--version").assert();

    assert
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")))
        .success();

    Ok(())
}
