<p align="center">
<img src="https://raw.githubusercontent.com/rustic-rs/assets/main/logos/readme_header_server.png" height="400" />
</p>
<p align="center"><b>REST server for rustic</b></p>
<p align="center">
<a href="https://crates.io/crates/rustic_server"><img src="https://img.shields.io/crates/v/rustic_server.svg" /></a>
<a href="https://docs.rs/rustic_server/"><img src="https://img.shields.io/docsrs/rustic_server?style=flat&amp;labelColor=1c1d42&amp;color=4f396a&amp;logo=Rust&amp;logoColor=white" /></a>
<a href="https://github.com/rustic-rs/rustic_server"><img src="https://img.shields.io/badge/license-Apache2.0/MIT-blue.svg" /></a>
<a href="https://crates.io/crates/rustic_server"><img src="https://img.shields.io/crates/d/rustic_server.svg" /></a>
<p>
<p align="center">
<a href="https://github.com/rustic-rs/rustic_server/actions/workflows/nightly.yml"><img src="https://github.com/rustic-rs/rustic_server/actions/workflows/nightly.yml/badge.svg" /></a>
<a href="https://www.gnu.org/licenses/agpl.txt"><img src="https://www.gnu.org/graphics/agplv3-88x31.png" height="20"/></a>
</p>

# ⚠️ This project is in early development and not yet ready for production use

For now, expect bugs, breaking changes, and a lot of refactoring.

Please feel free to contribute to this project, we are happy to help you getting
started. Join our [Discord](https://discord.gg/WRUWENZnzQ) to get in touch with
us.

## About

A REST server built in rust for use with rustic and rustic.

Works pretty similar to [rest-server](https://github.com/restic/rest-server).
Most features are already implemented.

## Contact

| Contact       | Where?                                                                                        |
| ------------- | --------------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/rustic-rs/rustic_server/issues)                            |
| Discord       | [![Discord](https://dcbadge.vercel.app/api/server/WRUWENZnzQ)](https://discord.gg/WRUWENZnzQ) |
| Discussions   | [GitHub Discussions](https://github.com/rustic-rs/rustic/discussions)                         |

## Are binaries available?

Yes, you can find them [here](https://rustic.cli.rs/docs/nightly_builds.html).

## Installation

You can install `rustic-server` using `cargo`:

```console
cargo install rustic_server
```

or you can download the binaries from the
[releases page](https://github.com/rustic-rs/rustic_server/releases).

## Usage

After installing `rustic-server`, you can start the server with the following
command:

```console
rustic-server serve
```

For more information, please refer to the
[`rustic-server` usage documentation](https://github.com/rustic-rs/rustic_server/blob/main/USAGE.md).

## Contributing

Tried rustic-server and not satisfied? Don't just walk away! You can help:

- You can report issues or suggest new features on our
  [Discord server](https://discord.gg/WRUWENZnzQ) or using
  [Github Issues](https://github.com/rustic-rs/rustic_server/issues/new/choose)!

Do you know how to code or got an idea for an improvement? Don't keep it to
yourself!

- Contribute fixes or new features via a pull requests!

Please make sure, that you read the
[contribution guide](https://rustic.cli.rs/docs/contributing-to-rustic.html).

## Minimum Rust version policy

This crate's minimum supported `rustc` version is `1.70.0`.

The current policy is that the minimum Rust version required to use this crate
can be increased in minor version updates. For example, if `crate 1.0` requires
Rust 1.20.0, then `crate 1.0.z` for all values of `z` will also require Rust
1.20.0 or newer. However, `crate 1.y` for `y > 0` may require a newer minimum
version of Rust.

In general, this crate will be conservative with respect to the minimum
supported version of Rust.

# License

`rustic-server` is open-sourced software licensed under the
[GNU Affero General Public License v3.0 or later](./LICENSE).
