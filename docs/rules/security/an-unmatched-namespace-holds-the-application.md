A namespace matching no grant line holds what the application holds — the operator's ceiling. Denying
by default here would refuse to compile every program written before the table existed and force every
application to grant itself, which is ceremony charged to the common case.

That default is only safe because **nothing a package manager fetched is ever unmatched**: fetching
writes a grant line for every package in the resolved graph, direct and transitive, **including an
empty one**. The permissive default therefore applies only to code a human wrote or pasted, which is
the case where it is the right answer.

Hand-vendored code is the residue, and it is reported rather than closed: an audit warns on a
namespace whose autoload root lies under a directory the grant table never mentions. A warning is the
correct strength, because the same shape describes a legitimate second source tree of the
application's own.

**The application's authority includes its defaults**
(`rule:config/no-configuration-file-is-a-complete-configuration`): on a CLI run it reads and writes
any file, reaches public hosts and loopback, and runs any program, and in server work it reads its
app folder and reaches public hosts, all with nothing written. Hand-vendored code with no grant line
holds all of that, so the audit warning is the mitigation for the wider defaults too, and the package
goal that builds the audit keeps it. Code a package manager fetched never holds a default: its line
is checked alone (`rule:security/package-authority-is-granted-one-line-at-a-time`).

A discovery glob happens to skip a directory such as `vendor/` because its name is not legal
`PascalCase`. That casing convention exists for another reason, and nothing here relies on it.

**Declined:**
- **Deny by default for an unmatched namespace.** It is safe and refuses to compile every program
  that exists. A grant line written for every fetched package, empty ones included, gives the same
  protection where it matters without charging the common case.

**Not on disk.** There is no fetch step and no audit that writes or checks these lines.
