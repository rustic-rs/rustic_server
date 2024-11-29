//! `RusticServer`
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

#![allow(non_local_definitions)]

pub mod acl;
pub mod application;
pub mod auth;
pub mod commands;
pub mod config;
pub mod context;
pub mod error;
pub mod handlers;
pub mod htpasswd;
pub mod log;
pub mod prelude;
pub mod storage;
pub mod typed_path;
/// Web module
///
/// implements a REST server as specified by
/// <https://restic.readthedocs.io/en/stable/REST_backend.html>
pub mod web;

#[cfg(test)]
pub mod testing;
