<p align="center">
<img src="https://raw.githubusercontent.com/rustic-rs/assets/main/logos/readme_header_config.png" height="400" />
</p>

# `rustic-server` Configuration Specification

This folder contains a few configuration files as an example.

`rustic-server` has a few configuration files:

- server configuration (`rustic_server.toml`)
- basic http credential authentication (`.htpasswd`)
- access control list (`acl.toml`)

## Server Config File - `rustic_server.toml`

This file may have any name, but requires valid toml formatting, as shown below.
A path to this file can be entered on the command line when starting the server.

```console
rustic-server serve --config/-c <path to config file>
```

### Server File format

```toml
[server]
listen = "127.0.0.1:8000"

[storage]
data-dir = "./test_data/test_repos/"
# The API for `quota` is not implemented yet, so this is not used
# We are also thinking about human readable sizes, like "1GB" and
# "1MB" etc., for deactivation of the quota, we might use `false`.
quota = 0

[auth]
disable-auth = false
htpasswd-file = "/test_data/test_repo/.htpasswd"

[acl]
disable-acl = false
acl-path = "/test_data/test_repo/acl.toml"
append-only = false

[tls]
disable-tls = false
tls-cert = "/test_data/test_repo/cert.pem"
tls-key = "/test_data/test_repo/key.pem"

[log]
log-level = "info"
log-file = "/test_data/test_repo/rustic.log"
```

## Access Control List File - `acl.toml`

Using the server configuration file, this file may have any name, but requires
valid toml formatting, as shown below.

A **path** to this file can be entered on the command line when starting the
server.

### ACL File format

```toml
# Format:
# [<repository_name>]
# <user> = <access_type>
# ... more users

[default] # Default repository
alex = "Read" # Alex can read
admin = "Modify" # admin can modify, so has full access, even delete

[alex] # a repository named 'alex'
alex = "Modify" # Alex can modify his own repository
bob = "Append" # Bob can append to Alex's repository
```

The `access_type` can have values:

- "Read" --> allows read only access
- "Append" --> allows addition of new files, including initializing a new repo
- "Modify" --> allows write-access, including delete of a repo

<!-- Todo: Describe "default" tag in the file. -->

# User Credential File - `.htpasswd`

This file is formatted as a vanilla `Apache .htpasswd` file.

Using the server configuration file, this file may have any name, but requires
valid formatting.

A **path** to this file can be entered on the command line when starting the
server. The server binary allows this file to be created from the command line.
Execute `rustic-server auth --help` for details. (This feature is not well
tested, yet. Please use with caution.)

You can also create this file manually, using the `htpasswd` command line tool.

```console
htpasswd -B -c <path to .htpasswd> username
```

# Configure `rustic_server` from the command line

It is also possible to configure the server from the command-line, and skip the
server configuration file. We recommend a configuration file for more complex
setups, though.

To see all options, use:

```console
rustic-server serve --help
```

They are all optional, and the server will use default values if not provided.
The server will also print the configuration it is using, so you can check if it
is correct when starting the server. For example:

```console
rustic-server serve --verbose
```
