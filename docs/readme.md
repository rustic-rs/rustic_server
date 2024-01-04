# `rustic_server`

A server allowing remote access to your backups

# Install

After building `rustic_server`, one binary is available:

- `rustic_server`: the web server

# `rustic_server` sub commands

## `rustic_server config`

`rustic_server config` will help you create a configuration file for the server
from scratch. It assumes a certain folder structure, but you can change that
later if you want. Just make sure that the central configuration file
`rustic_server.toml` points to the right files.

Folder structure assumed:

```
/<rustic_server_base_path>
                           /.htaccess 
                           /acl.toml
                           /rustic_server.toml
                           /repos/...
```

File/Folder content:

- `.htaccess`: file contains passwords for users having access to the
  repositories. Note since the filename starts with a "dot"; it is probably not
  visible when listing the content of the parent folder.
- `acl.toml`: file describes which user is allowed access to what repository,
  and with what access (append, modify, ...)
- `rustic_server.toml`: configuration file. Point the web server to this file,
  and the rest is "configured" :-)
- `repos`: A folder which contains the repositories with your backups.

Execute `rustic_server config`, and you get a few questions on the prompt.

Before starting, make sure `<rustic_server_base_path>` is writable for you. And
if you want secure HTTP with TLS, that you have the file location for the
certificats at hand. And that these certificats are readable for the server when
executing.

## `rustic_server auth`

`rustic server auth` allows you to change the password of users having access to
the web server. The `.htaccess` file is basically a list of users and their
(encrypted) passwords. It contains a few sub commands:

- Add: Adds one user to the list
- Update: Updates a known user from the list
- Delete: Deletes one user from the list
- List: Lists all user entries in the list

To list all known users:

```
rustic_server auth -c /<rustic_server_base_path>/rustic_server.toml List
```

Usage:

```
rustic_server auth -c /<rustic_server_base_path>/rustic_server.toml <sub-command> -u <user_name>
```

Note that the conifuguration path used points to the server configuration, not
to the `.htaccess`-file.

## `rustic_server serve`

`rustic_server serve` starts the web server.

Usage:

```
rustic_server -c /<rustic_server_base_path>/rustic_server.toml
```

It is also possible to enter more command line options to give all configuration
parameters that way. To find out all command line options, and their function;
use the command:

```
rustic_server serve  -h
```
