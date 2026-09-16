---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Connecting to a database"
description: "One database API, every statement prepared, three deny-by-default capabilities, and defaults that close what PHP leaves open."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/regex-html-and-introspection/
  label: "Regex, HTML and introspection"
next:
  link: /docs/rules/core-classes/running-a-statement/
  label: "Running a statement"
---

<p class="nv-section-lead">One database API, every statement prepared, three deny-by-default capabilities, and defaults that close what PHP leaves open.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">9</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#db-one-api"><code>Core\Db</code> is the only database API, and every statement it runs is prepared</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-connection-is-named">A connection is named in configuration or built from settings, and both memoize for the request</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-capabilities">Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-safe-connection-defaults">Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-drivers-are-an-enum">The five drivers are an enum with one <code>match</code> per entry point, not a <code>Driver</code> trait</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-unix-socket-path">A Unix-socket host is the string that deployment already holds, and MSSQL refuses one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#server-version-is-what-the-server-said"><code>serverVersion</code> answers what the server said during the handshake, and never spends a round trip</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-connection-busy-state">Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-statement-members">Five members run a statement, results are buffered by default, and only one of them streams</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="db-one-api">

## `Core\Db` is the only database API, and every statement it runs is prepared

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-one-api"><code>core-classes/db-one-api</code></a>
</div>

`Core\Db` is the only way to reach a database. There is no procedural twin, no second object API, and
no escaping function: the query-text parameter refuses `tainted` while bound parameters accept it
freely ([`core-classes/db-parameters`](/docs/rules/core-classes/running-a-statement/#db-parameters "One placeholder is one value, and expanding a list into IN is written with Core\Db::inList")), and an escaper would be a second, weaker answer to a
question already settled.

There is **no `prepare` step**. `query` and `execute` take SQL and parameters together, and each
connection holds an LRU cache of server-side prepared statements keyed by SQL text plus expansion
arity, sized by `statement_cache` in the connection's config block. Two spellings of one operation
would buy nothing the cache does not already provide.

The cost is recorded rather than hidden: on MySQL and MariaDB a statement's first execution in a
request costs two round trips and cached re-executions cost one, while PostgreSQL's extended protocol
pays nothing extra. Emulated prepares — string interpolation inside the driver — do not exist in any
form.

Permanently refused, and not deferred: multi-statement queries, which are the amplifier that turns
one injection into a compromise; `LOAD DATA LOCAL INFILE`; `PDO::quote` and
`mysqli_real_escape_string`; by-reference parameter or column binding; connection-level
`lastInsertId` state; `PDO::ATTR_*`; and `PDO::inTransaction`. Query builders, ORMs and migration
tooling are not `Core` at all.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no PDO, no mysqli, no pgsql and no sqlite3 twin, no <code>quote</code>/<code>real_escape_string</code>, no emulated prepares and no multi-statement query</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-parameters" title="One placeholder is one value, and expanding a list into IN is written with Core\Db::inList"><code>core-classes/db-parameters</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-prepared-statement-binds-by-name-and-by-position.nvst"><code>tests/conformance/core/db-a-prepared-statement-binds-by-name-and-by-position.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-query-takes-its-values-beside-the-statement.nvst"><code>tests/conformance/core/db-query-takes-its-values-beside-the-statement.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-query-refuses-a-tainted-statement-and-binds-one-freely.nvst"><code>tests/conformance/core/db-query-refuses-a-tainted-statement-and-binds-one-freely.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-quote-identifier-refuses-what-would-need-delimiting.nvst"><code>tests/conformance/core/db-quote-identifier-refuses-what-would-need-delimiting.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-connection-is-named">

## A connection is named in configuration or built from settings, and both memoize for the request

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-connection-is-named"><code>core-classes/db-connection-is-named</code></a>
</div>

`Core\Db::connect` resolves a root-owned `[db.<name>]` block, so a credential never appears in
program source, never enters the repository, and is rotated without a deploy. `Core\Db::open` covers
what a name cannot — one database per tenant, chosen at request time — from a `Db\Settings` value.

Both memoize per request: `connect` keys on the *name*, `open` on a hash of *every* settings field.
Keying `connect` on the name keeps two identically-configured blocks as two connections, because an
operator who wrote two blocks meant two. Hashing all of `open`'s fields means a second call differing
only in `timeout` gets its own connection rather than silently inheriting the first caller's.
`{shared: false}` bypasses memoization, which is how a program writes an audit row that must survive
a rollback or holds session-scoped state off the shared connection.

`Db\Settings` is a discriminated union over enum-case types rather than one loose shape: SQLite takes
a `path` and has no `host`, so a `host` on a SQLite settings literal is a compile error rather than a
silently ignored field.

A connection is released by the runtime at request teardown — the job a destructor would have done,
done by the arena instead. `close()` releases one early, and the name is then free for the next
`connect`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A credential is never a DSN string in program source, and there is no destructor to close a connection — the request teardown does it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-safe-connection-defaults" title="Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value"><code>core-classes/db-safe-connection-defaults</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0064.md">record 0064</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0103.md">record 0103</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-connection-names-its-driver-and-is-open-until-closed.nvst"><code>tests/conformance/core/db-a-connection-names-its-driver-and-is-open-until-closed.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-settings-are-checked-against-the-one-arm-they-select.nvst"><code>tests/conformance/core/db-settings-are-checked-against-the-one-arm-they-select.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-settings-require-the-keys-of-the-arm-they-select.nvst"><code>tests/conformance/core/db-settings-require-the-keys-of-the-arm-they-select.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-closed-name-is-free-for-the-next-connect.nvst"><code>tests/conformance/core/db-a-closed-name-is-free-for-the-next-connect.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-closed-connection-refuses-every-member-that-needs-it.nvst"><code>tests/conformance/core/db-a-closed-connection-refuses-every-member-that-needs-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-capabilities">

## Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-capabilities"><code>core-classes/db-capabilities</code></a>
</div>

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
accept `tainted` freely, and `password` is `secret tainted string`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Opening a connection is a granted act rather than a library call, and a hostname reaching <code>Settings.host</code> may not be <code>tainted</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a> <a href="/docs/rules/core-classes/schemas/#schema-apply-capability" title="Planning is an ordinary read, and applying takes db.schema under a member named for its risk"><code>core-classes/schema-apply-capability</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-safe-connection-defaults" title="Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value"><code>core-classes/db-safe-connection-defaults</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-connect-refuses-a-name-the-grant-does-not-cover.nvst"><code>tests/conformance/core/db-connect-refuses-a-name-the-grant-does-not-cover.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst"><code>tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-asks-the-grant-about-a-sqlite-path-and-not-a-host.nvst"><code>tests/conformance/core/db-open-asks-the-grant-about-a-sqlite-path-and-not-a-host.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-connect-judges-its-options-before-it-consults-the-grant.nvst"><code>tests/conformance/core/db-connect-judges-its-options-before-it-consults-the-grant.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-safe-connection-defaults">

## Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-safe-connection-defaults"><code>core-classes/db-safe-connection-defaults</code></a>
</div>

Three defaults close holes PHP leaves open, and none of the three is configurable to the unsafe
value.

**`LOCAL INFILE` is off**, with no option to enable it — a server that asks the client to send it a
file gets nothing. **TLS defaults to `VerifyFull`** on a TCP connection; PHP's `pdo_pgsql` defaults to
`sslmode=prefer`, which silently connects in plaintext when the server says so, and a settings
literal naming a weaker mode does not compile. **The connection charset is forced to UTF-8**
(`utf8mb4` on MySQL and MariaDB), so text columns arrive as valid UTF-8 and
[`types/string-is-utf8`](/docs/rules/types/text-and-literal-types/#string-is-utf8 "A string is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster")'s guarantee holds by construction rather than by hope.

The cost is one connection option a deployment cannot turn down, which is the point: an unsafe
default that can be restored is an unsafe default a misconfigured deployment still has.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>LOCAL INFILE</code> cannot be enabled, TLS verifies fully rather than <code>prefer</code>, and the charset is forced to UTF-8</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-column-types" title="Every column has one natural Novis type, and the requested type converts losslessly or throws"><code>core-classes/db-column-types</code></a> <a href="/docs/rules/types/text-and-literal-types/#bytes" title="bytes is a primitive peer to string for data that carries no encoding"><code>types/bytes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-refuses-a-tls-mode-weaker-than-verify-full.nvst"><code>tests/conformance/core/db-open-refuses-a-tls-mode-weaker-than-verify-full.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-drivers-are-an-enum">

## The five drivers are an enum with one `match` per entry point, not a `Driver` trait

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-drivers-are-an-enum"><code>core-classes/db-drivers-are-an-enum</code></a>
</div>

The five drivers are one enum — PostgreSQL, MySQL, MariaDB, SQL Server, SQLite — each variant owning
its own state machine, error-code table and reset. `Core\Db`'s entry points `match` once.

Three reasons, in priority order. **The set is closed**: a new backend is a decision record, not a
plugin, and a sandboxed extension cannot host one anyway, since a pool is the thing a sandbox
boundary cannot hold — open-set extensibility is the one property a trait buys and this design does
not want it. **A trait wide enough for all five would be half unimplemented**: `executeMany` is one
prepare and N executions on four drivers and a bulk protocol message on the fifth, a prepare is a
round trip on two and free on one, and a reset is four command sequences and a rollback on the fifth.
A trait method four drivers implement by returning an error is a lie the type system helped tell.
**Static dispatch on the request path** costs no allocation and no indirect call.

Where a signature repeats five times, it repeats. What keeps the five honest is not a type but one
assertion set run against five real servers. What is shared is the half with no driver in it — the
placeholder rewriter, the statement cache, the pool, the Novis side of the type map, and error-kind
normalisation, whose per-driver code tables are *data* each driver supplies rather than behaviour it
overrides.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-crate-boundary" title="The wire lives in nvs-db below the standard library, where a codec is borrowed and a state machine is written"><code>core-classes/db-crate-boundary</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-unix-socket-path" title="A Unix-socket host is the string that deployment already holds, and MSSQL refuses one"><code>core-classes/db-unix-socket-path</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0003.md">record 0003</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-connection-names-its-driver-and-is-open-until-closed.nvst"><code>tests/conformance/core/db-a-connection-names-its-driver-and-is-open-until-closed.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-unix-socket-path">

## A Unix-socket host is the string that deployment already holds, and MSSQL refuses one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-unix-socket-path"><code>core-classes/db-unix-socket-path</code></a>
</div>

Where an operator wrote a local endpoint, the string they write is the one their deployment already
holds. For `driver = "postgres"` the `host` names the **directory** and the socket is derived —
`/var/run/postgresql` with `port = 5432` is `/var/run/postgresql/.s.PGSQL.5432` — because that is
what libpq, `psql`, PDO and every Postgres tool take. MySQL and MariaDB take the socket **file**,
because their socket has no naming convention to derive one from, and that too is the string those
deployments already hold.

**MSSQL refuses a path.** TDS has no `AF_UNIX` transport, so the driver reports it as a target it
does not speak rather than as a file it could not open. SQLite is untouched: its path *is* the
database.

The cost is one piece of protocol trivia per driver, encoded where that driver's trivia belongs.

Each driver's answer lives with that driver. `crates/nvs-db/src/mysql.rs` and
`crates/nvs-db/src/maria.rs` open the socket file as written, through one `socket_endpoint` rather
than two identical ones; `crates/nvs-db/src/pg.rs` derives the engine's own name in its own. All
three dial over the second arm of `crate::conn::Endpoint` — the address-or-path a driver's `connect`
takes — and answer a server the same way over either, which is what
`a_driver_answers_the_same_over_either_transport` asserts. `crates/nvs-db/src/tds/mod.rs` refuses a
path in `TdsTarget::resolve`, as `BlockError`'s `NoSocketTransport`.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/mysql.rs"><code>crates/nvs-db/src/mysql.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/maria.rs"><code>crates/nvs-db/src/maria.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/pg.rs"><code>crates/nvs-db/src/pg.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/src/tds/mod.rs"><code>crates/nvs-db/src/tds/mod.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="server-version-is-what-the-server-said">

## `serverVersion` answers what the server said during the handshake, and never spends a round trip

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#server-version-is-what-the-server-said"><code>core-classes/server-version-is-what-the-server-said</code></a>
</div>

`serverVersion` answers what the server said during the handshake, kept on the connection, and no
driver ever issues a statement to learn it.

| Driver | What is answered | Where it comes from |
|---|---|---|
| PostgreSQL | the `server_version` string verbatim | the `ParameterStatus` at startup |
| MySQL | the greeting's banner verbatim | the handshake packet's version field |
| MariaDB | the greeting's banner verbatim, which is how a MariaDB server says it is one | the same field |
| SQL Server | `major.minor.build`, decimal and unpadded — `16.0.4125` | `LOGINACK`'s numeric triple |
| SQLite | the library version, as `sqlite3_libversion` reports it | the linked library, not a server |

A server's own string is answered **unchanged**, suffix and all: the value exists to say what is on
the other end, and a driver that tidies it hides the thing being asked about. Where the server sends
numbers rather than a string, the table above fixes the one spelling, so a program comparing versions
across drivers compares one shape. The member never answers `null` — each of the five has an answer —
and it costs one short string per open wire connection, and nothing at all on SQLite, whose answer is
the linked library's and belongs to the binary rather than to a connection.

A `SELECT version()` is refused: it spends a round trip on the request path to learn something the
connection was already told. [`core-classes/schema-plan`](/docs/rules/core-classes/schemas/#schema-plan "Every plan step carries a grade and its complete SQL, and an unknown grade grades up")'s grader is keyed on the server version
and reads it off the connection it already holds, so the same refusal keeps a grade from costing a
statement of its own.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a> <a href="/docs/rules/core-classes/schemas/#schema-plan" title="Every plan step carries a grade and its complete SQL, and an unknown grade grades up"><code>core-classes/schema-plan</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/db_stream.rs"><code>crates/nvs-stdlib/tests/db_stream.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-server-version-answers-what-the-connection-kept.nvst"><code>tests/conformance/core/db-server-version-answers-what-the-connection-kept.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-server-version-costs-no-statement-so-a-held-connection-answers-it.nvst"><code>tests/conformance/core/db-server-version-costs-no-statement-so-a-held-connection-answers-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-connection-busy-state">

## Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-connection-busy-state"><code>core-classes/db-connection-busy-state</code></a>
</div>

Whether a connection may take a new statement is a field on the connection, not on the stream: a
connection is busy whether or not its socket is readable, and SQLite must answer the same
`LogicError` with no stream underneath it at all. The four states are `Idle` (the wire is at a
message boundary), `Executing` (a buffered statement is in flight), `Streaming` (rows remain unread,
so a second statement is [`core-classes/db-streaming`](/docs/rules/core-classes/running-a-statement/#db-streaming "A streaming result holds its connection until it is drained, and a second statement on it throws")'s refusal) and `Poisoned` (the wire is
*not* at a known message boundary). It is a plain cell with no atomic and no lock, because a task
never migrates and a connection is owned by one request at a time.

**A poisoned connection is closed, never reset, and never returned to the pool.** The reset is a
security boundary because a connection carrying one request's state into another's is a cross-tenant
leak; a `RESET ALL` written into the middle of an unfinished message is not a reset but a fragment of
one request's protocol stream that the next request will read as its own. Draining first would mean
trusting a length prefix that has already proven untrustworthy. Closing costs one handshake and is
the only answer that is provable.

An abandoned stream is not automatically poison: a driver that can cancel and drain deterministically
returns to `Idle` and pools the connection, and one that cannot poisons it. That choice is per
driver, in the driver.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-streaming" title="A streaming result holds its connection until it is drained, and a second statement on it throws"><code>core-classes/db-streaming</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-closed-connection-refuses-every-member-that-needs-it.nvst"><code>tests/conformance/core/db-a-closed-connection-refuses-every-member-that-needs-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-statement-members">

## Five members run a statement, results are buffered by default, and only one of them streams

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-statement-members"><code>core-classes/db-statement-members</code></a>
</div>

Five members run a statement. `query` and `queryAs<T>` answer a buffered `Db\Rows`, `execute` answers
a `Db\Write` carrying `affected`, `changed` and `lastId`, `executeMany` answers a count, and
`stream`/`streamAs<T>` answer an `Iterable` ([`core-classes/db-streaming`](/docs/rules/core-classes/running-a-statement/#db-streaming "A streaming result holds its connection until it is drained, and a second statement on it throws")).

Buffering is the default because memory ranks last, and because the alternative breaks the commonest
loop in web programming — reading rows and writing per row — on a connection-busy rule that only
surfaces at run time. The rule is uniform across drivers even though SQL Server's MARS could lift it,
so that code written against one driver runs on all five.

`executeMany` does **not** open a transaction of its own; a caller who wants all-or-nothing writes
one, which composes with savepoint nesting and with retry. It is one prepare and N executions on
every driver, including MariaDB: its bulk-execute command ends on a refusal where the loop attempts
every later set, and it cannot route a statement that answers with a result set, so taking it would
mean one program leaving different rows in two servers. The price is N round trips where one command
would do, and it is recorded here rather than hidden.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no unbuffered-query flag and no per-driver difference in what a second statement on a busy connection does</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-streaming" title="A streaming result holds its connection until it is drained, and a second statement on it throws"><code>core-classes/db-streaming</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-column-types" title="Every column has one natural Novis type, and the requested type converts losslessly or throws"><code>core-classes/db-column-types</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state" title="Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset"><code>core-classes/db-connection-busy-state</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-query-answers-a-rows-and-not-an-array.nvst"><code>tests/conformance/core/db-query-answers-a-rows-and-not-an-array.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-execute-answers-a-write-and-not-a-count.nvst"><code>tests/conformance/core/db-execute-answers-a-write-and-not-a-count.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-execute-many-answers-a-count-and-not-a-write.nvst"><code>tests/conformance/core/db-execute-many-answers-a-count-and-not-a-write.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-execute-many-takes-a-set-per-execution.nvst"><code>tests/conformance/core/db-execute-many-takes-a-set-per-execution.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-query-as-answers-rows-at-the-class-it-was-written-with.nvst"><code>tests/conformance/core/db-query-as-answers-rows-at-the-class-it-was-written-with.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-write-tells-an-absent-count-from-a-zero-one.nvst"><code>tests/conformance/core/db-write-tells-an-absent-count-from-a-zero-one.nvst</code></a></dd></div></dl>

</div>
