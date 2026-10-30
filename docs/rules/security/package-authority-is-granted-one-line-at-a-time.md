A package **declares** what it requests, split into required and optional. That declaration is
documentation and an upper bound on itself; it grants nothing. The **application grants explicitly,
one line at a time**, and a namespace granted nothing holds nothing. Adding a dependency prints every
capability requested by that package *and its whole transitive subgraph* before a human writes
anything, so the authority a new dependency brings is visible in one diff at the moment it is
introduced rather than discoverable by audit later.

What this buys, stated plainly: a fully malicious package that reaches the compiler cannot open a
socket, read a file, spawn a process, reach a database or spawn a script unless a human wrote its name
in a grant line. The compromise of a transitive dependency degrades from *arbitrary action with the
process's authority* to *arbitrary computation with no authority at all*. The one read that needs no
line is a package reading its own listed files, whose bytes its digest already fixes
(`rule:packaging/a-package-carries-only-its-source-and-what-it-lists`).

Grant lines are friction, and the friction is the point. Some users will grant everything without
reading, the way some run containers as root; the design cannot prevent that, only make it visible and
diffable. What it prevents is authority arriving without anybody writing anything.

**A default never reaches a package.** The application's own code holds the run's defaults with no
line written (`rule:config/no-configuration-file-is-a-complete-configuration`), and a package is
checked against its own grant line alone, intersected with what the application wrote. A default is
never part of a grant line, so widening the application's defaults widens no package. Hand-vendored
code with no line is the exception, and the audit warning of
`rule:security/an-unmatched-namespace-holds-the-application` is its mitigation.

A package may lower its own limits and never raise one. A `Core\Config::set` call under a package's
namespace that raises a `[limits]` key returns `false`, even where the application's own code may
raise it: on a CLI run with no `[limits.hard] memory` written, the application raises `memory` freely
(`rule:config/ceilings-are-their-own-directives`). A raise would let a dependency take memory the
application never chose to spend.

A package can never be the code that declares data safe. Only a `Core` member removes `tainted`
(`rule:security/launderers-are-sink-named`), so a package can compute over a `tainted` value and return
it still `tainted`.

**Declined:**
- **Ambient capabilities: the process is granted, and every package inherits.** That is what every
  mainstream package system does, and it is how a compromised transitive dependency becomes a breach.
  No incumbent can retrofit the refusal.
- **A package inherits the application's defaults.** It would hand every dependency the files, hosts
  and programs a first program needs, which is the ambient authority above under another name.

`nvs pkg fetch` writes the grant lines (`crates/nvs-cli/src/package/fetch.rs`), and the compiler
checks each call against them (`crates/nvs-types/src/capability.rs`). Before `nvs pkg add` writes, it
prints every package of the new dependency's subgraph and every capability each one requests
(`crates/nvs-cli/src/package/report.rs`).
