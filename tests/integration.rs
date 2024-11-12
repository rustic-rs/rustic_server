//! Integration tests for the rustic_server
//
// # Notes
//
// * https://restic.readthedocs.io/en/latest/030_preparing_a_new_repo.html#rest-server
// * https://restic.readthedocs.io/en/latest/100_references.html#rest-backend
// * https://github.com/restic/rest-server
//
// use anyhow::Result;
// use assert_cmd::Command;
// use predicates::prelude::{predicate, PredicateBooleanExt};
// use dircmp::Comparison;

// pub fn server_runner() -> Result<Command> {
//     let password = "test";
//     let repo_dir = temp_dir.path().join("repo");

//     let mut runner = Command::new(env!("CARGO_BIN_EXE_rustic-server"));

//     runner
//         .arg("");

//     Ok(runner)
// }
