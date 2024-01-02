# `rustic_server`
A server allowing remote access to your backups

# Install

After building `rustic_server`, additional binaries are available:
 - `rustic_server`: the web server
 - `rustic_server_config`: allows you to configure the server
 - `rustic_server_htaccess`: allows you to change the passwords for users

# `rustic_server_config`

`rustic_server_config` will help you create a configuration file for the server.
It assumes a certain folder structure, but you can change that later if you want.
Just make sure that the central configuration file `rustic_server.toml` points to
the right files.

Folder structure assumed:

```
/<rustic_server_base_path>/
                           .htaccess 
                           acl.toml
                           rustic_server.toml
                           repos/...
```

File/Folder content:
 - `.htaccess`: file contains passwords for users having access to the repositories.
 - `acl.toml`: file contains which user is allowed access to what repository, and with what access (append, modify, ...)
 - `rustic_server.toml`: configuration file. Point the web server to this file, and the rest is "configured" :-)
 - `repos`: A folder which contains the repositories with your backups. 


Execute `rustic_server_config`, and you get a few questions on the prompt. 

Before starting, make sure `<rustic_server_base_path>` is writable for you. And if you want secure HTTP with TLS, 
that you have the file location for the certificats at hand. And that these certificats are readable for the 
server when executing.

# `rustic_server_htaccess`
Ofcause you man want to change passwords at some time in the future, or add new users having access
to the repositories. The passwords can be changed using ``rustic_server_htaccess``.

Execute: 
```
rustic_server_htaccess -c /<rustic_server_base_path>/rustic_server.toml
```
