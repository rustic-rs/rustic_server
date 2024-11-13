//! RusticServer
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa

#![forbid(unsafe_code)]
#![warn(
    // unreachable_pub, // frequently check
    // TODO: Activate and create better docs
    // missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications,
    // TODO: Activate if you're feeling like fixing stuff 
    // clippy::pedantic,
    // clippy::correctness,
    // clippy::suspicious,
    // clippy::complexity,
    // clippy::perf,
    clippy::nursery,
    bad_style,
    dead_code,
    improper_ctypes,
    missing_copy_implementations,
    missing_debug_implementations,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    trivial_numeric_casts,
    unused_results,
    unused_extern_crates,
    unused_import_braces,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true,
    clippy::cast_lossless,
    clippy::default_trait_access,
    clippy::doc_markdown,
    clippy::manual_string_new,
    clippy::match_same_arms,
    clippy::semicolon_if_nothing_returned,
    clippy::trivially_copy_pass_by_ref
)]

pub mod acl;
pub mod application;
pub mod auth;
pub mod commands;
pub mod config;
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
/// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
pub mod web;

#[cfg(test)]
pub mod test_helpers;
