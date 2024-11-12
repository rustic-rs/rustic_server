//! RusticServer
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

pub mod acl;
pub mod application;
pub mod auth;
pub mod commands;
pub mod config;
pub mod error;
pub mod handlers;
pub mod htaccess;
pub mod log;
pub mod prelude;
pub mod storage;
pub mod typed_path;
/// Web module
///
/// implements a REST server as specified by
/// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
pub mod web;

#[cfg(test)]
pub mod test_helpers;
