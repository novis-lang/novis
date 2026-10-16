An entry's optional `limits` table is a sub-cap on the run's budget, and its optional `grants` table
narrows the run's capabilities. Both are **narrowing only**: an entry cannot raise `memory` above the
deployment's `[limits]`, and cannot grant a capability the deployment's `[capabilities]` withheld or
widen a scope it narrowed. A scheduled run cannot widen anything, for the same reason no program can
(`rule:security/no-runtime-grant`) — the root-owned file is the ceiling, and a block inside it is not a
second authority.

The ticker builds both when it arms the entry and applies them to each fire's isolate before its
first statement, which is the same narrowing a `spawn script` site writes and not a scheduler's own
mechanism. Nothing checks either table against the deployment first: a sub-cap wider than what is in
force leaves the inherited ceiling standing, and a name the deployment withheld is still refused at
the door, so the application is the check.

**`grants` narrows by capability name, and a scope written beside it narrows nothing.**
`grants = {net.connect = ["reports.internal"]}` holds the run to `net.connect` and leaves the hosts
the deployment named standing — the entry reaches no host `[capabilities]` withheld, and takes none
away from itself either. That is what a narrowing is everywhere, a spawn site's `grants:` included:
a list of names, asked beside the configuration rather than instead of it. A second channel carrying
scopes would make an entry a second place a capability's scope is resolved, which is the thing
`rule:security/capability-check-at-the-door` keeps to one.
