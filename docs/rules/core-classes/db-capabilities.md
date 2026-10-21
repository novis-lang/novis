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

A SQLite file named in a configuration block is covered by `db.connect`. A path a program supplies
through `open` asks the file rule alone, with its default for the run
(`rule:config/no-configuration-file-is-a-complete-configuration`), and never `db.open`: `fs.read`
is required, and where `fs.write` is not granted the file opens with SQLite's read-only flag, so a
write throws SQLite's read-only error. A CLI run therefore opens any file read-write, and server work
opens a file in its app folder read-only. A read-only and a read-write open of one file are two pool
keys. `:memory:` and the empty path are private to their connection and ask nothing. A `file:` URI
can name any file and mode, so it asks `db.open`, `fs.read` and `fs.write`. The path is a path sink.
An ungranted name throws naming the capability. What the split buys is that the set of network
databases a deployment can reach is enumerable from its configuration; what it costs is that code
building a network connection target at run time does not run until the operator adds a `db.open`
grant.

**Declined:**
- **A `db.open` grant for a SQLite file.** A file is what `fs.read` and `fs.write` already govern,
  and a second grant for the same path is a second answer to one question.
- **A host pattern under `db.connect` or `net.connect`.** `db.connect` names configuration blocks, not
  hosts, and a pattern under `net.connect` would widen the reachable set to every name an attacker
  can get into that zone's DNS, which is what the pinned address of
  `rule:http-server/allow-url-pins-the-address` exists to bound.
