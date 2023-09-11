<p align="center">
<img src="https://media.githubusercontent.com/media/rustic-rs/docs/main/assets/readme_header_server.png" height="400" />
</p>
<p align="center"><b>REST server for rustic</b></p>

<!-- <p align="center">
<a href="https://crates.io/crates/rustic-rs"><img src="https://img.shields.io/crates/v/rustic-rs.svg" /></a>
<a href="https://docs.rs/rustic-rs/"><img src="https://img.shields.io/docsrs/rustic-rs?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://raw.githubusercontent.com/rustic-rs/rustic/main/"><img src="https://img.shields.io/badge/license-Apache2.0/MIT-blue.svg" /></a>
<a href="https://crates.io/crates/rustic-rs"><img src="https://img.shields.io/crates/d/rustic-rs.svg" /></a>
<p> -->

<p align="center">
<a href="https://github.com/rustic-rs/rustic_server/actions/workflows/nightly.yml"><img src="https://github.com/rustic-rs/rustic_server/actions/workflows/nightly.yml/badge.svg" /></a>
</p>

## About

A REST server built in rust for use with rustic and restic.

Works pretty similar to [rest-server](https://github.com/restic/rest-server).
Most features are already implemented.

## Contact

| Contact       | Where?                                                                                 |
| ------------- | -------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/rustic-rs/rustic_server/issues)                     |
| Discord       | [![](https://dcbadge.vercel.app/api/server/WRUWENZnzQ)](https://discord.gg/WRUWENZnzQ) |
| Discussions   | [GitHub Discussions](https://github.com/rustic-rs/rustic/discussions)                  |

## Dependencies

Is built using [tide](https://github.com/http-rs/tide),
[tide-rustls](https://github.com/http-rs/tide-rustls) and
[tide-http-auth](https://github.com/chrisdickinson/tide-http-auth).

## Are binaries available?

Yes, you can find them [here](https://rustic.cli.rs/docs/nightly_builds.html).

## Missing features / TODOs

- Tests
- CI pipeline
- An option `max-size`
- support for prometheus
- Storage part: Error handling etc

## Additional feature

Allows to give ACLs im TOML format, use option `--acl`

Example TOML file:

```toml
# default sets ACL for the repo without explicit path (and for the repo under path "default", if exists)
[default]
alex = "Read"
admin = "Modify"

[alex]
alex = "Modify"
bob = "Append"
```
