A package **declares** what it requests, split into required and optional. That declaration is
documentation and an upper bound on itself; it grants nothing. The **application grants explicitly,
one line at a time**, and a namespace granted nothing holds nothing. Adding a dependency prints every
capability requested by that package *and its whole transitive subgraph* before a human writes
anything, so the authority a new dependency brings is visible in one diff at the moment it is
introduced rather than discoverable by audit later.

What this buys, stated plainly: a fully malicious package that reaches the compiler cannot open a
socket, read a file, spawn a process, reach a database or spawn a script unless a human wrote its name
in a grant line. The compromise of a transitive dependency degrades from *arbitrary action with the
process's authority* to *arbitrary computation with no authority at all*.

Grant lines are friction, and the friction is the point. Some users will grant everything without
reading, the way some run containers as root; the design cannot prevent that, only make it visible and
diffable. What it prevents is authority arriving without anybody writing anything.

A package can never be the code that declares data safe. Only a `Core` member removes `tainted`
(`rule:security/launderers-are-sink-named`), so a package can compute over a `tainted` value and return
it still `tainted`.

**Declined:**
- **Ambient capabilities: the process is granted, and every package inherits.** That is what every
  mainstream package system does, and it is how a compromised transitive dependency becomes a breach.
  No incumbent can retrofit the refusal.

**Not on disk.** There is no package manager, no manifest, and no grant line writer in the tree.
