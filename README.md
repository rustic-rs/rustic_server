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

## About

A REST server built in rust for use with rustic and restic.

Works pretty similar to [rest-server](https://github.com/restic/rest-server).
Most features are already implemented.

## Contact

| Contact       | Where?                                                                                        |
| ------------- | --------------------------------------------------------------------------------------------- |
| Issue Tracker | [GitHub Issues](https://github.com/rustic-rs/rustic_server/issues)                            |
| Discord       | [![Discord](https://dcbadge.vercel.app/api/server/WRUWENZnzQ)](https://discord.gg/WRUWENZnzQ) |
| Discussions   | [GitHub Discussions](https://github.com/rustic-rs/rustic/discussions)                         |

## Dependencies

Is built using [tide](https://github.com/http-rs/tide),
[tide-rustls](https://github.com/http-rs/tide-rustls) and
[tide-http-auth](https://github.com/chrisdickinson/tide-http-auth).

## Are binaries available?

Yes, you can find them [here](https://rustic.cli.rs/docs/nightly_builds.html).

## Additional feature

Allows to give ACLs im TOML format, use option `--acl`

Example TOML file:

```toml
# default sets ACL for the repo without explicit path
# (and for the repo under path "default", if exists)
[default]
alex = "Read"
admin = "Modify"

[alex]
alex = "Modify"
bob = "Append"
```

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

# License

`rustic-server` is open-sourced software licensed under the
[GNU Affero General Public License v3.0 or later](./LICENSE).
