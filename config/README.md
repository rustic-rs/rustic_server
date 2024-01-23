# `rustic_server` configuration

This folder contains a few configuration files as an example.

`rustic_server` has a few configuration files:

- access control list (acl.toml)
- server configuration (rustic_server.toml)
- basic http credential authentication (.htaccess)

See also the rustic configuration, described in:
https://github.com/rustic-rs/rustic/tree/main/config

## `acl.toml`

This file may have any name, but requires valid toml formatting.

Format:

```
[<repository_name>]
<user> <access_type>
... more users

... more repositories
```

The `access_type` can have values:

- "Read" --> allows read only access
- "Append" --> allows addition of new files, including initializing a new repo
- "Modify" --> allows write-access, including delete of a repo

Todo: Describe "default" tag in the file.

## `rustic_server.toml`

This file may have any name, but requires valid toml formatting.

File format:

```
[server]
host_dns_name = <ip_address> | <dns hostname>
port = <port number>

[repos]
storage_path = <local file system path containing repos>

[authorization]
auth_path = <path to .htaccdss file, including filename>
use_auth = <skip authorization if false>

[access_control]
acl_path = <path to the acl file, including filename>
private_repo = <skip access control if false>
append_only = <limit to append, regardless of the ACL file content>
```

On top of some additional configuration items, the content of this file points
to the `acl.toml`, and `.htaccess` files.

## `.htaccess`

This is a vanilla `Apache` `.htacces` file.

# Configure `rustic_server` from the command line

It is also possible to configure the server from the command line, and skip the
configuration file.

To see all options, use:

```
rustic_server --help
```
