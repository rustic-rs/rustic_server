# Test data folder
The test data folder contains data required for testing the server.

FIXME: Future move to a container to also allow rustic interfaces to be 
tested abainst the rustic server?

# Basic files for test access to a repository

### `HTACCESS`

File governing the access to the server. Without access all is rejected.

htaccess file has one entry:

- user: test
- password: test_pw

### `acl.toml`

Definition which user from the HTACCESS file has what privileges on
which repository.

Most used seems to be the `test_repo` with members
- user: test
- Access level: Read
But there are 2 more in the file.

### `rustic_server.toml`

Server configuration file which allows the `rustic_server` to be started with
only a pointer to this file. This file points to:

- HTACCESS file <br> Note: that the HTACCESS file does not need to be a hidden
  file. Rustic will use the file you point to.
- acl.toml file
- path to: repository (where all your backups are)
- path to: https TLS certiciate and key file
- dns_hostname, and port to listen to

This file allows a server to be started.

### `rustic.toml`

Configuration file for the `rustic` commands. Start as:

```
rustic -P <path_to>/test.toml <rustic_command>
```

In the configuration folder there is an example given. Adapt to your configuration.
To make use of the `test_repo`, the file has to contain the following credentials:

```
[repository]
repository = "rest:http://test:test_pw@localhost:8000/test_repo"
password = "test_pw"
```

# Repository

There are 2 folders with test data. One source folder, and a repository folder which 
should be the folder that contains the rustic backup of the source folder.