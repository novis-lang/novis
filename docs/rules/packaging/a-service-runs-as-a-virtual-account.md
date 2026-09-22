The default identity is `LocalSystem`, the local system account every hand-registered Windows service
gets, so an operator who installs with no `--account` sees the same log-on tab `sc create` gives them.
`SYSTEM` and `NT AUTHORITY\SYSTEM` are accepted as that account's other names and stored as the SCM
spells it. It holds every right already, so an install grants it nothing and an uninstall revokes
nothing from it.

`--account` takes another identity: `NT SERVICE\<name>`, the per-service virtual account the SCM
creates and owns, with a per-service SID, no password to rotate or leak and no interactive logon; or a
domain account, with the password prompted rather than taken from the command line
(`rule:packaging/the-installer-is-a-sink`). For those, install grants the account read on the config
and read/write on the cache and log directories, and nothing further; a cache or log directory that is
not there is created by the install, because such an account holds nothing on the parent and so could
never create it itself. An uninstall revokes the same entries, and reads a path that is no longer
there as already revoked.
