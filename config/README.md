# `rustic_server` configuration

This folder contains a few configuration files as an example.

`rustic_server` has a few configuration files:

- access control list (acl.toml)
- server configuration (rustic_server.toml)
- basic http credential authentication (.htpasswd)

See also the rustic configuration, described in:
<https://github.com/rustic-rs/rustic/tree/main/config>

# Server config file `rustic_server.toml`

This file may have any name, but requires valid toml formatting, as shown below.
A path to this file can be entered on the command line when starting the server.

File format:

```
[server]
host_dns_name = <ip_address> | <dns hostname>
port = <port number>
common_root_path = <absolute path to your repo>

[repos]
# Absolute file path will be: /common_root_path>/<repo_folder>
# if <common_root_path> is empty, an absolute path to a folder is expected here 
storage_path = <repo_folder>

[authorization]
# Absolute file path will be: /common_root_path>/<auth filename>
# if <common_root_path> is empty, an absolute path to a file is expected here 
auth_path = <auth filename>
use_auth = <skip authorization if false>

[access_control]
# Absolute file will be: /common_root_path>/<acl filename>
# if <common_root_path> is empty, an absolute path to a file is expected here
acl_path = <acl filename>
private_repo = <skip access control if false>
append_only = <limit to append, regardless of the ACL file content>
```

# Access control list file `acl.toml`

Using the server configuration file, this file may have any name, but requires
valid toml formatting, as shown below.

A **path** to this file can be entered on the command line when starting the
server.

File format:

```
[<repository_name>]
<user> <access_type>
... more users

[<other_repository>]
<user> <access_type>
... more users
```

The `access_type` can have values:

- "Read" --> allows read only access
- "Append" --> allows addition of new files, including initializing a new repo
- "Modify" --> allows write-access, including delete of a repo

Todo: Describe "default" tag in the file.

# user credential file `.htpasswd`

This file is formatted as a vanilla `Apache` `.htpasswd` file.

Using the server configuration file, this file may have any name, but requires
valid formatting.

A **path** to this file can be entered on the command line when starting the
server. In that case the file name has to be `.htpasswd`.

The server binary allows this file to be created from the command line. Execute
`rustic_server --help` for details.

# Configure `rustic_server` from the command line

It is also possible to configure the server from the command line, and skip the
server configuration file.

To see all options, use:

```
rustic_server --help
```
