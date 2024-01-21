# HTACCESS

File governing the access to the server. Without access all is rejected.

htaccess file has one entry:

- user: test
- password: test_pw

# acl.toml

Finegrained definition which user from the HTACCESS file has what privileges on
which repository.

Access control list file

- user: test
- Access level: Read

# rustic_server.toml

Server configuration file which allows the `rustic_server` to be started with
only a pointer to this file. This file points to:

- HTACCESS file <br> Note: that the HTACCESS file does not need to be a hidden
  file. Rustic will use the file you point to.
- acl.toml file
- path to: repository (where all your backups are)
- path to: https TLS certiciate and key file
- dns_hostname, and port to listen to

# test.toml

Configuration file for the `rustic` commands. Start as:

```
rustic -P <path_to>/test.toml <rustic_command>
```
