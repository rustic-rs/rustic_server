# Test data folder

The test data folder contains data required for testing the server.

## Basic files for test access to a repository

### `.htpasswd`

File governing the access to the server. Without access all is rejected.

The `.htpasswd` file has three entries:

- rustic:rustic
- restic:restic
- hurl:hurl

### `acl.toml`

Definition which user from the HTACCESS file has what privileges on which
repository.

Check [here](config/README.md) for more information.

### `rustic_server.toml`

Server configuration file.

### `rustic.toml`

Configuration file for the `rustic` commands. Start as:

```console
rustic -P <path_to>/test.toml <rustic_command>
```

In the configuration folder there is an example given. Adapt to your
configuration. To make use of the `test_repo`, the file has to contain the
following credentials:

```toml
[repository]
repository = "rest:http://rustic:rustic@localhost:8000/ci_repo"
password = "rustic"
```

### `certs` directory

Contains the test certificates for the server.

## Source folder for Testing

There is a source folder with test data.

## Storage folder for Testing

There is a storage folder with test data. It is used to store the data for the
server. The data is stored in the `tests/generated/test_storage` directory.
