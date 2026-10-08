**Every file the configuration reads must be owned by the account the runtime runs as or by root, and
must not be group- or world-writable.** Its containing directory must pass the same check, ownership
included and not only the mode: an entry can be replaced by whoever can write the directory holding
it, whatever the file's own bits say. A failure is a refusal to start (`E0607`), naming the path and
the mode, and it is re-run on every reload.

The configuration grants capabilities, so whoever can write any file in the tree can grant themselves
every one of them, and a running server publishes a saved file without a restart. The cache directory
already gets this refusal, and a file that grants `process.exec` cannot have less protection than a
directory of compiled code.

**On Windows the equivalent is the DACL**: the owner is the runtime account, `BUILTIN\Administrators`
or `NT AUTHORITY\SYSTEM`, and no *effective* write right — data, append, EA, attributes, `DELETE`,
`WRITE_DAC` or `WRITE_OWNER` — reaches `Everyone`, `Authenticated Users`, `BUILTIN\Users`,
`BUILTIN\Guests` or `ANONYMOUS LOGON`. Effective rights rather than the presence of an ACE, so a deny
entry counts. Windows grants `Authenticated Users` modify rights by default on a non-system drive's
root, so a configuration kept under such a path refuses until that inheritance is broken — that default
*is* the hole this rule closes.

**One directory is checked without its parent: the default data folder**, `.nvsdata` beside the
running binary. Its parent is the binary's own directory, and an account that can write there can
already replace the binary, so examining it guards nothing the binary does not — while on a stock
Windows drive it would make the default folder unusable everywhere. A folder `--data` names, and every
path the configuration names, keeps the full check, parent included. A file inside the data folder is
checked as any file is, with the private data folder as its directory.

Where the check runs is `rule:config/the-ownership-check-runs-where-it-can-be-answered`.
