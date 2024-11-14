# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0](https://github.com/rustic-rs/rustic_server/compare/v0.1.1...v0.2.0) - 2024-11-14

### Other

- update readme
- [**breaking**] move to axum - Part II ([#56](https://github.com/rustic-rs/rustic_server/pull/56))

## [0.1.1] - 2024-01-09

### Bug Fixes

- Nightly builds, exclude arm64 darwin build until issue Publishing
  aarch64-apple-darwin failed #6 is fixed
- Update rust crate toml to 0.8
- Deserialization with newest toml
- Clippy
- Remove unmaintained `actions-rs` ci actions
- Update rust crate clap to 4.4.10
  ([#37](https://github.com/rustic-rs/rustic_server/issues/37))
- Update github action to download artifacts, as upload/download actions from
  nightly workflow were incompatible with each other
- Don't unwrap in bin
- Imports

### Documentation

- Update readme, fix manifest
- Add continuous deployment badge to readme
- Fix typo in html element
- Add link to nightly
- Add link to nightly downloads in documentation
- Remove CI Todo
- Remove To-dos from Readme
- Break line in toml code for usability
- Update changelog
- Rewrite contributing remark
- Fix list indent
- Add contributing
- Remove tide remarks from readme

### Features

- Pr-build flag to build artifacts for a pr manually if needed

### Miscellaneous Tasks

- Add ci
- Nightly builds
- Update header link
- Add release pr workflow
- Add caching
- Add signature and shallow clones to nightly
- Declutter and reorganize
- Remove lint from ci workflow and keep it separate, replace underscore in
  workflow files
- Rebase and extract action to own repository
- Use create-binary-artifact action
- Put action version to follow main branch while action is still in development
- Switch ci to rustic-rs/create-binary-artifact action
- Switch rest of ci to rustic-rs/create-binary-artifact action
- Change license
- Fix workflow name for create-binary-artifact action, and check breaking
  changes package dependent
- Decrease build times on windows
- Fix github refs
- Set right package
- Use bash substring comparison to determine package name from branch
- Fix woggly github action comparison
- Add changelog generation
- Initialize cargo release, update changelog
- Add dev tooling
- Run git-cliff with latest tag during release
- Remove comment from cargo manifest
- Change workflow extensions to yml
- Add triaging of issues
- Run release checks also on release subbranches
- Add maskfile
- Update changelog
- Run workflow on renovate branches
- Add merge queue checks
- Add cargo deny
- Relink to new image location
- Add binstall support
- Build nightly with rsign signed binaries
- Update public key
- Support rsign signature
- Remove special os-dependent linker/compiler settings
- Update cross ci
- Check if nightly builds for arm64 darwin builds work now
- Arm64 on darwin still fails
- Add x86_64-pc-windows-gnu target
- Compile dependencies with optimizations in dev mode
- Add results to ci
- Lockfile maintenance
- Run actions that need secrets.GITHUB_TOKEN only on rustic-rs org
- Update dtolnay/rust-toolchain
- Update taiki-e/install-action
- Update rustsec/audit-check
- Netbsd nightly builds fail due to missing execinfo, so we don't build on it
  for now
- Upgrade dprint config
- Activate automerge for github action digest update
- Activate automerge for github action digest update
- Automerge lockfile maintenance
- :debug
- Update to latest axum and apply fixes
- Reactivate audit workflow
- Remove OnceCell dep and set rust-version
- Remove justfile

### Refactor

- Refactor to library and server binary
- Begin refactor to axum
- [**breaking**] Moving to axum
  ([#40](https://github.com/rustic-rs/rustic_server/issues/40))
- Use own errors throughout library part
- State better which file is not able to be read
- More error handling stuff

### Testing

- Fix config tests
