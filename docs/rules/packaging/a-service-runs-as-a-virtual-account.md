The default identity is `NT SERVICE\<name>` — a virtual account the SCM creates and owns, with a
per-service SID, no password to rotate or leak, and no interactive logon. Install grants that SID read
on the config, read/write on the cache and log directories, and nothing further; the account can read
its configuration and write its cache and log, and cannot write its own binary. A cache or log directory
that is not there is created by the install, because the account holds nothing on the parent and so
could never create it itself. An uninstall revokes the same entries, and reads a path that is no longer
there as already revoked.

`--account` takes a domain identity for a deployment that needs one, with the password prompted rather
than taken from the command line (`rule:packaging/the-installer-is-a-sink`). `LocalSystem` is never the
default and must be written out as `--account SYSTEM`.
