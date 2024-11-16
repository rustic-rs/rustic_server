# Usage

You can start the server with the following command:

```console
rustic-server serve
```

## Defaults

### Storage

By default the server persists backup data in the OS temporary directory
(`/tmp/rustic` on Linux/BSD and others, in `%TEMP%\\rustic` in Windows, etc).

**If `rustic-server` is launched using the default path, all backups will be
lost**. To start the server with a custom persistence directory and with
authentication disabled:

```sh
rustic-server --path /user/home/backup --no-auth
```

`rustic-server` uses exactly the same directory structure as local backend, so
you should be able to access it both locally and via HTTP, even simultaneously.

### Authentication (Basic)

To authenticate users (for access to the `rustic-server`), the server supports
using a `.htpasswd` file to specify users. By default, the server looks for this
file at the root of the persistence directory, but this can be changed using the
`--htpasswd-file` option. You can create such a file by executing the following
command (note that you need the `htpasswd` program from Apache's http-tools). In
order to append new user to the file, just omit the `-c` argument. Only bcrypt
and SHA encryption methods are supported, so use -B (very secure) or -s
(insecure by today's standards) when adding/changing passwords.

```sh
htpasswd -B -c .htpasswd username
```

If you want to disable authentication, you must add the `--no-auth` flag. If
this flag is not specified and the `.htpasswd` cannot be opened, `rustic-server`
will refuse to start.

### Transport Layer Security (TLS)

By default the server uses HTTP protocol. This is not very secure since with
Basic Authentication, user name and passwords will be sent in clear text in
every request. In order to enable TLS support just add the `--tls` argument and
specify private and public keys by `--tls-cert` and `--tls-key`.

Signed certificate is normally required by `restic` and `rustic`, but if you
just want to test the feature you can generate password-less unsigned keys with
the following command:

```sh
openssl req -newkey rsa:2048 -nodes -x509 -keyout private_key -out public_key -days 365 -addext "subjectAltName = IP:127.0.0.1,DNS:yourdomain.com"
```

Omit the `IP:127.0.0.1` if you don't need your server be accessed via SSH
Tunnels. No need to change default values in the openssl dialog, hitting enter
every time is sufficient.

To access this server via `restic` use `--cacert public_key`, meaning with a
self-signed certificate you have to distribute your `public_key` file to every
`restic` client.

### Access Control List (ACL)

To prevent your users from accessing each others' repositories, you may use the
`--private-repos` flag in combination with an ACL file.

This server supports `ACL`s to restrict access to repositories. The ACL file is
formatted in TOML and can be specified using the `--acl-path` option. More
information about the ACL file format can be found in the `acl.toml` file in the
`config` directory. If the ACL file is not specified, the server will allow all
users to access all repositories.

For example, user "foo" using the repository URLs
`rest:https://foo:pass@host:8000/foo` or `rest:https://foo:pass@host:8000/foo/`
would be granted access, but the same user using repository URLs
`rest:https://foo:pass@host:8000/` or `rest:https://foo:pass@host:8000/foobar/`
would be denied access. Users can also create their own sub repositories, like
`/foo/bar/`.

## Append-Only Mode

The `--append-only` mode allows creation of new backups but prevents deletion
and modification of existing backups. This can be useful when backing up systems
that have a potential of being hacked.

## Credits

This project is based on the
[rest-server](https://github.com/restic/rest-server) project by
[restic](https://restic.net). This document is based on the
[README.md](https://github.com/restic/rest-server/blob/e35c6e39d9c8d658338e1d9a0e4a57a50e151957/README.md)
of the rest-server project.
