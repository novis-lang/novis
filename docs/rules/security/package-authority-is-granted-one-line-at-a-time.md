A package **declares** what it requests, split into required and optional. That declaration is
documentation and an upper bound on itself; it grants nothing. The **application grants explicitly,
one line at a time**, and a namespace granted nothing holds nothing. Adding a dependency prints every
capability requested by that package *and its whole transitive subgraph* before a human writes
anything, so the authority a new dependency brings is visible in one diff at the moment it is
introduced rather than discoverable by audit later.

What this buys, stated plainly: a fully malicious package that reaches the compiler cannot open a
socket, read a file, read the process environment or a deployment secret, spawn a process, reach a
database or spawn a script unless a human wrote its name in a grant line. The compromise of a transitive dependency degrades from *arbitrary action with the
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

A package may lower its own limits and change nothing else. A `Core\Config::set` call under a
package's namespace returns `false` for every key outside `limits.*`: the mode, `[http]`, `[log]`,
`[debug]`, `[errors]` and every other block are the application's. Under `limits.*` it returns
`false` for a raise, even where the application's own code may raise the key: on a CLI run with no
`[limits.hard] memory` written, the application raises `memory` freely
(`rule:config/ceilings-are-their-own-directives`). A raise would let a dependency take memory the
application never chose to spend, and a change to any other block would let it weaken a policy the
application chose, such as its cookies' flags. An application that wants a package's policy calls
`set` itself.

A package's `Core\Config::restore` undoes only what a package set. When a package lowers a key the
application also set, the request keeps the application's value under the package's, and the
package's `restore` puts it back. A key no package set is left as it is, so a package never removes
a value the application set. The application's own `set` or `restore` of the key makes the entry the
application's again. A method reference carries no word for the call site, so it acts as a package's
call wherever it is written. **What it spends:** one optional string per key a package set, per
request, freed with the request. Declined: telling one package from another. A limit is no wall
between two packages that share one request's budget.

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
