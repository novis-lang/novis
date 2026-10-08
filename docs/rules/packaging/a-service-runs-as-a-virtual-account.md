The default identity is `LocalSystem`, the local system account every hand-registered Windows service
gets, so an operator who installs with no `--account` sees the same log-on tab `sc create` gives them.
`SYSTEM` and `NT AUTHORITY\SYSTEM` are accepted as that account's other names and stored as the SCM
spells it. It holds every right already, so an install grants it nothing and an uninstall revokes
nothing from it.

`--account` takes another identity: `NT SERVICE\<name>`, the per-service virtual account the SCM
creates and owns, with a per-service SID, no password to rotate or leak and no interactive logon; or a
domain account, with the password prompted rather than taken from the command line
(`rule:packaging/the-installer-is-a-sink`). For those, install grants the account **read** on the
service's data folder, on its `nvs.toml` and on every `--config` file, and **read/write** on the data
folder's `cache/`, `tmp/`, `lsp/` and `logs/` only, each inherited only inside its own subfolder; a log
directory, `[opcache] file_cache_dir` or `[io] temp_root` outside those four gets its own read/write
grant, and nothing further is granted. **The service account never writes its own configuration.** A
directory the grant names that is not there is created by the install, because such an account holds
nothing on the parent and so could never create it itself. An uninstall finds the data folder from the
binary the platform holds for the service, revokes the same entries, and reads a path that is no
longer there as already revoked.

On Linux the unit runs as root unless `--account` names a user, which must exist — one that does not
is an `E0654` before anything is installed — and the same list is spelled as owners and modes: the
data folder becomes root's `0750` and its `nvs.toml` root's `0640`, both in the account's primary
group, and each read/write path, created if missing, the account's own `0700`. A `--config` file
outside the data folder keeps its owner and mode, and the install warns when the account cannot read
it. Every path the service reads is then owned by root or by itself and writable by no group, which
is what `rule:config/ownership-is-the-trust-boundary` checks at boot. An uninstall gives each path back
to root with the owner's bits of its mode only.
