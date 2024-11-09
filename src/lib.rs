pub mod acl;
pub mod auth;
pub mod commands;
pub mod config;
pub mod error;
pub mod handlers;
pub mod log;
pub mod storage;
pub mod typed_path;
/// Web module
///
/// implements a REST server as specified by
/// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
pub mod web;

#[cfg(test)]
pub mod test_helpers;
