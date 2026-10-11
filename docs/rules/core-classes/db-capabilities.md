Three deny-by-default capabilities sit in front of a database. `db.connect` names the configuration
blocks a program may open. `db.open` names the hosts dynamic settings may reach, and is the only one
of the three taking a `*.` pattern, which matches at a label boundary — `*.tenants.internal` grants
`a.b.tenants.internal` and grants neither `tenants.internal` nor `evil-tenants.internal`, and a bare
`*` grants nothing. `db.schema` names blocks a program may issue DDL to, which is a strictly larger
act than any query: DDL takes no parameters, so it is a sink nothing can be bound through.

An address an operator wrote into root-owned configuration carries the same authority that granted
the capability, so a `connect`-named endpoint is pre-approved and is not additionally checked against
the outbound policy's denied ranges — which matters, because a database lives at `10/8`, a container
network or `127.0.0.1`. A `db.open` target is program-supplied and stays subject to that policy in
full.

`Settings.host` refuses `tainted` and has **no launderer**: no string check can establish that a
hostname is safe to send credentials to, since a malicious server can answer any query with a
`LOCAL INFILE` request. `Core\Taint::assertTrusted` is the only way through. `database` and `user`
accept `tainted` freely, being length-prefixed protocol fields rather than parsed text, and `password`
is `secret tainted string`.

A SQLite file named in a configuration block is covered by `db.connect`; a path a program supplies
through `open` additionally needs `fs.read` and `fs.write`, and is a path sink. An ungranted name
throws naming the capability. What the split buys is that the set of databases a deployment can
reach is enumerable from its configuration; what it costs is that code building a connection target
at run time does not run until the operator adds a `db.open` grant.

**Declined:**
- **A host pattern under `db.connect` or `net.connect`.** `db.connect` names configuration blocks, not
  hosts, and a pattern under `net.connect` would widen the reachable set to every name an attacker
  can get into that zone's DNS, which is what the pinned address of
  `rule:http-server/allow-url-pins-the-address` exists to bound.
