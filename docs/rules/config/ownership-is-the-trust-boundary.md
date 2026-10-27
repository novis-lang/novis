**Every file the configuration reads must be owned by the account the runtime runs as or by root, and
must not be group- or world-writable.** Its containing directory must pass the same check, ownership
included and not only the mode: an entry can be replaced by whoever can write the directory holding
it, whatever the file's own bits say. A failure is a refusal to start (`E0607`), naming the path and
the mode, and it is re-run on every reload. Its help is the one command that repairs that failure,
with the path, the owner or the group's SID filled in: `chown` or `chmod go-w` on Unix, and on
Windows `icacls /setowner *S-1-5-32-544` for an owner or `/inheritance:d` and `/remove:g <SID>` for a
group. The help is built only for a breach, so a path that passes costs nothing more.

**The directory that holds each link on the path as given is checked too.** A reload reads a root
through its links again (`rule:config/the-resolved-root-is-announced-and-stored`), so whoever can write
the directory a link sits in chooses the file the next reload reads. Every prefix of the path that is a
link has its parent checked, a `current` link above the file as well as the file's own link. A path
with no link is checked as the file and its directory alone. A link's directory has one waiver the
file's directory lacks: **a root-owned directory on a read-only mount skips the mode test.** That is
the kubelet's ConfigMap volume, measured on `kind`: `0777 root:root` on a read-only `ext4` mount,
holding `override.toml -> ..data/override.toml` and `..data -> ..<timestamp>`. No account in the pod
can write it, and only root on the node changes it. A directory another account owns, or one on a
writable mount, keeps the mode test. The waiver is `trust::platform::check_link_holder`.

The configuration grants capabilities, so whoever can write any file in the tree can grant themselves
every one of them, and a running server publishes a saved file without a restart. The cache directory
already gets this refusal, and a file that grants `process.exec` cannot have less protection than a
directory of compiled code.

**On Windows the equivalent is the DACL**: the owner is the runtime account, `BUILTIN\Administrators`
or `NT AUTHORITY\SYSTEM`, and no *effective* write right — data, append, EA, attributes, `DELETE`,
`WRITE_DAC` or `WRITE_OWNER` — reaches `Everyone`, `Authenticated Users`, `BUILTIN\Users`,
`BUILTIN\Guests` or `ANONYMOUS LOGON`. The two ACL rights are in the mask because either one buys all
the others in a second step. Effective rights rather than the presence of an ACE, so a deny entry
counts and an inherited grant is seen. Windows grants `Authenticated Users` modify rights by default on a non-system drive's
root, so a configuration kept under such a path refuses until that inheritance is broken — that default
*is* the hole this rule closes.

**On Unix, a file or directory on a read-only mount passes the owner test**: `statvfs` reports
`ST_RDONLY` for its file system. No account on this host can change it through that mount, and that
is the whole of what the owner test guards. A container's `-v ./app:/app:ro` is exactly this case:
the files belong to the host's account, which is not the container's, and refusing them would refuse
the deployment the Docker guide teaches. The mode test still runs there, so a group- or
world-writable mode on a read-only mount is refused: the mode says the tree is meant to be shared,
and the same tree mounted read-write later would be served. A path whose mount cannot be asked keeps
the owner test. Windows has no read-only mount to waive.

Docker Desktop on Windows can report a bind mount's files as `0777`, whatever the NTFS ACL says, and
then the mode test refuses them even under `:ro`. The guide tells a Windows user to copy the
configuration into the image instead. The mode test is not waived for it: a mode is the only answer
the container can read, and a check that trusted `0777` would trust every mount.

**One directory is checked without its parent: the default data folder**, `.nvsdata` beside the
running binary. Its parent is the binary's own directory, and an account that can write there can
already replace the binary, so examining it guards nothing the binary does not — while on a stock
Windows drive it would make the default folder unusable everywhere. A folder `--data` names, and every
path the configuration names, keeps the full check, parent included. A file inside the data folder is
checked as any file is, with the private data folder as its directory. Novis creates the data folder
private to its account and never re-permissions one that exists: it checks it.

**The account is a value the check takes, not one it reads.** `nvs serve` judges for its own account,
and a command that sets up or reports on a service judges for the account the service runs as, through
the same function: `trust::check_as` with a `trust::Account`, a uid on Unix and a SID on Windows. On
Unix a number is a uid and a name comes from the password database. On Windows `LocalSystem` is the
well-known SID, `NT SERVICE\<name>` is computed as Windows computes it (`S-1-5-80-` and the SHA-1 of
the upper-case name in UTF-16LE), because that account does not resolve until the service exists, and
any other name goes through `LookupAccountNameW`. A breach for a named account names it: "neither the
account `LocalSystem` nor an administrative one". `resolve::JudgedFor` resolves a tree that way.

**Declined:**
- **The full check on the default data folder.** It is correct in principle and unusable on a stock
  Windows drive, and it protects only against an account that could replace the binary anyway.
- **Waiving the mode test on a read-only mount too.** A writable mode under `:ro` is either a
  mistake or Docker Desktop's `0777`, and the container cannot tell the two apart. Only a
  root-owned directory that holds a link has that waiver, because a ConfigMap volume is exactly that.
- **The mode test on a link's directory with no waiver.** It refuses every Kubernetes ConfigMap,
  whose volume directory is `0777 root:root` on a read-only mount.
- **The waiver for a link's directory another account owns.** Docker Desktop's `0777` bind mount
  belongs to the host's account, and that is the case the mode test is kept for.
- **A configuration key that turns the check off.** It would be written in the file the check
  guards, so whoever can write the file could turn it off.

Where the check runs is `rule:config/the-ownership-check-runs-where-it-can-be-answered`.
