# tide-http-auth

A bring-your-own-user-type [Tide][ref-tide] middleware for implementing
pluggable basic and bearer auth for authorization headers.

Use this crate if:

- You want to implement `Authorization: Bearer <tktktk>` auth.
- You want to implement `Authorization: Basic` auth.

See the [examples][ref-examples] for more.

# License

Apache-2.0

[ref-tide]: https://github.com/http-rs/tide
[ref-examples]: ./examples/
