---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Running a statement"
description: "One placeholder is one value. Columns have natural types, transactions are closures, and one error kind spans every driver."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/connecting-to-a-database/
  label: "Connecting to a database"
next:
  link: /docs/rules/core-classes/schemas/
  label: "Schemas and convergence"
---

<p class="nv-section-lead">One placeholder is one value. Columns have natural types, transactions are closures, and one error kind spans every driver.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">4</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#db-parameters">One placeholder is one value, and expanding a list into <code>IN</code> is written with <code>Core\Db::inList</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-literal-query-checking">A literal query is checked while compiling, and no vendor's SQL grammar is ever parsed</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-column-types">Every column has one natural Novis type, and the requested type converts losslessly or throws</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-streaming">A streaming result holds its connection until it is drained, and a second statement on it throws</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-stream-parks-its-read-on-the-connection">A stream's read state is parked on the connection on every driver, and a buffer is never the answer</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-transactions">A transaction is a closure, and <code>Transaction</code> is a <code>Queryable</code> rather than a second query surface</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-error">One <code>DbError</code> carries a normalised kind across every driver, and never a bound parameter</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#db-crate-boundary">The wire lives in <code>nvs-db</code> below the standard library, where a codec is borrowed and a state machine is written</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="db-parameters">

## One placeholder is one value, and expanding a list into `IN` is written with `Core\Db::inList`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-parameters"><code>core-classes/db-parameters</code></a>
</div>

The params argument is a single `array<mixed>`. A list-keyed array means positional `?` placeholders;
a string-keyed one means `:name`; mixing the two in one call throws `LogicError`. Both are rewritten
for the driver, and a `:name` used twice binds one value once, which positional form cannot express.
The rewriter skips string literals, comments and PostgreSQL's `::` cast and jsonb operators, which is
why `??` escapes a literal question mark.

**One parameter is always one value.** A list bound to a placeholder is a single value — a PostgreSQL
array column, a JSON document — and `Core\Db::inList($values)` is the explicit marker that expands
into a parenthesised placeholder list. Automatic expansion was refused on the explicit-typing rule
rather than on how common array columns are: it would make the SQL *text* depend on a runtime value's
type, so a `mixed` that turned out to be a list would reshape the query instead of failing.

`inList([])` throws. An empty list means "match nothing" inside `IN` and "match everything" inside
`NOT IN`, the rewriter cannot tell which it is in, and picking one silently is worse than making the
caller branch. Expansion changes the statement's arity, so the statement cache keys on it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An array bound to a placeholder is one value rather than a silently expanded list, and an empty <code>inList</code> throws instead of guessing</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-one-api" title="Core\Db is the only database API, and every statement it runs is prepared"><code>core-classes/db-one-api</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-in-list-marks-a-run-of-bound-values.nvst"><code>tests/conformance/core/db-in-list-marks-a-run-of-bound-values.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-in-list-refuses-an-empty-list.nvst"><code>tests/conformance/core/db-in-list-refuses-an-empty-list.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-in-list-carries-every-element-type.nvst"><code>tests/conformance/core/db-in-list-carries-every-element-type.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-prepared-statement-binds-by-name-and-by-position.nvst"><code>tests/conformance/core/db-a-prepared-statement-binds-by-name-and-by-position.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-literal-query-checking">

## A literal query is checked while compiling, and no vendor's SQL grammar is ever parsed

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-literal-query-checking"><code>core-classes/db-literal-query-checking</code></a>
</div>

A literal SQL argument is validated while checking, under the closed intrinsic-literal list:
placeholder count against a literal params array, positional-versus-named consistency, an
unterminated string literal, and a refused second statement. A literal `Db::open` host matching no
`db.open` grant is likewise a check-time diagnostic, since configuration is read at boot on the
machine that compiles.

Full per-dialect SQL parsing is **not** done, and never will be: it would mean maintaining four
vendors' grammars in the front end forever. That refusal covers DDL as well as queries, which is why
[`core-classes/schema-introspection`](/docs/rules/core-classes/schemas/#schema-introspection "An existing database is read by catalog query, and no DDL parser exists at any tier") reads an existing database by catalog query rather than by
parsing the `CREATE TABLE` a server prints.

What this costs is that a column name typo survives to run time. What it buys is that the compiler
never has to be right about a dialect it does not own.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-parameters" title="One placeholder is one value, and expanding a list into IN is written with Core\Db::inList"><code>core-classes/db-parameters</code></a> <a href="/docs/rules/core-classes/schemas/#schema-introspection" title="An existing database is read by catalog query, and no DDL parser exists at any tier"><code>core-classes/schema-introspection</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals" title="A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact"><code>expressions/intrinsic-literals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0057.md">record 0057</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-stream-is-checked-while-compiling-like-every-other-statement.nvst"><code>tests/conformance/core/db-stream-is-checked-while-compiling-like-every-other-statement.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-open-refuses-a-settings-literal-that-names-no-driver.nvst"><code>tests/conformance/core/db-open-refuses-a-settings-literal-that-names-no-driver.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-column-types">

## Every column has one natural Novis type, and the requested type converts losslessly or throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-column-types"><code>core-classes/db-column-types</code></a>
</div>

Every column has one natural Novis type, and `get()` and `toArray()` answer it. Asking for another
type converts when that is lossless and throws `DbError` otherwise: MySQL's `TINYINT(1)` is naturally
`int` and reads as `bool` on request with a stored `7` throwing; a `BIGINT UNSIGNED` past `i64::MAX`
reads as `uint` and throws for `int`; a `DECIMAL` refuses a `float` field. One rule settles what would
otherwise be a list of special cases.

The map itself is fixed: integers to `int` or `uint`, `DECIMAL`/`NUMERIC`/`MONEY` to `decimal`, text
families to `tainted string`, binary families to `tainted bytes`, `DATE`/`TIME`/`TIMESTAMPTZ`/`UUID`
to their `Core` types, PostgreSQL arrays to `array<T>`, and everything with no Novis type — `inet`,
ranges, `hstore`, geometry, `interval` — to `tainted string`. **JSON is not auto-decoded**, because
MariaDB's `JSON` is a `LONGTEXT` alias and so is not reliably detectable from metadata at all. A
zone-less `DATETIME` reads in a zone the connection declares, sent to the server as a numeric offset
so `CURRENT_TIMESTAMP` agrees.

There is deliberately **no universal string**: the universal path is `get(): mixed` plus the
language's own `as`, so `Core\Db` never grows a second stringification table. Every value a row
yields is `tainted` where its type can carry it, which closes stored injection by the same mechanism
as reflected. The same table is read the other way by [`core-classes/schema-vocabulary-is-closed`](/docs/rules/core-classes/schemas/#schema-vocabulary-is-closed "The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Nothing is a string because the text protocol made it one — a <code>DECIMAL</code> is a <code>decimal</code>, a <code>BIGINT UNSIGNED</code> is a <code>uint</code>, and <code>TINYINT(1)</code> holding <code>7</code> refuses to read as <code>bool</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a> <a href="/docs/rules/core-classes/schemas/#schema-vocabulary-is-closed" title="The schema vocabulary is five constructs shared by all five backends, with no raw escape hatch"><code>core-classes/schema-vocabulary-is-closed</code></a> <a href="/docs/rules/types/declarations-and-numbers/#decimal" title="decimal is an exact scalar of 96 mantissa bits and a scale of 0 to 28"><code>types/decimal</code></a> <a href="/docs/rules/types/declarations-and-numbers/#uint" title="uint is a distinct unsigned 64-bit integer, and it costs no bytes a value did not already hold"><code>types/uint</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0054.md">record 0054</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0036.md">record 0036</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-row-reads-a-column-by-type-and-answers-nullable.nvst"><code>tests/conformance/core/db-row-reads-a-column-by-type-and-answers-nullable.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-typed-row-throws-naming-the-column.nvst"><code>tests/conformance/core/db-a-typed-row-throws-naming-the-column.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-column-type-is-a-case-and-not-a-vendor-name.nvst"><code>tests/conformance/core/db-column-type-is-a-case-and-not-a-vendor-name.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-sqlite-column-reads-back-as-the-type-its-schema-declared.nvst"><code>tests/conformance/core/db-a-sqlite-column-reads-back-as-the-type-its-schema-declared.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-rows-answers-the-types-the-results-table-names.nvst"><code>tests/conformance/core/db-rows-answers-the-types-the-results-table-names.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-streaming">

## A streaming result holds its connection until it is drained, and a second statement on it throws

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-streaming"><code>core-classes/db-streaming</code></a>
</div>

`stream` and `streamAs<T>` read a result set in constant memory, and they **hold the connection until
drained**. A second statement attempted on a streaming connection throws `LogicError` naming both
fixes: `->all()`, or a `{shared: false}` connection.

There is no `{chunk?: uint}` option, and its absence is a refusal rather than an unlanded feature.
The PostgreSQL portal is opened with a row count of *every row* and one `DataRow` is read per step, so
a streamed result already crosses on a single round trip while the client holds one row — the memory
a chunk size exists to bound is already one row, and the option could only spend latency to buy
nothing.

**Both members answer on all five drivers**, and a driver never substitutes a buffer for a walk it
cannot park. This rule is the member's contract — constant memory, the connection held, the second
statement refused — and the read state each driver leaves between two steps is
[`core-classes/a-stream-parks-its-read-on-the-connection`](/docs/rules/core-classes/running-a-statement/#a-stream-parks-its-read-on-the-connection "A stream's read state is parked on the connection on every driver, and a buffer is never the answer"), which is the mechanism under it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state" title="Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset"><code>core-classes/db-connection-busy-state</code></a> <a href="/docs/rules/core-classes/running-a-statement/#a-stream-parks-its-read-on-the-connection" title="A stream's read state is parked on the connection on every driver, and a buffer is never the answer"><code>core-classes/a-stream-parks-its-read-on-the-connection</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/db_stream.rs"><code>crates/nvs-stdlib/tests/db_stream.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-stream-is-checked-while-compiling-like-every-other-statement.nvst"><code>tests/conformance/core/db-stream-is-checked-while-compiling-like-every-other-statement.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-stream-refuses-a-tainted-statement.nvst"><code>tests/conformance/core/db-stream-refuses-a-tainted-statement.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-stream-parks-its-read-on-the-connection">

## A stream's read state is parked on the connection on every driver, and a buffer is never the answer

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-stream-parks-its-read-on-the-connection"><code>core-classes/a-stream-parks-its-read-on-the-connection</code></a>
</div>

A streaming statement's read state is split from the borrow and parked on the connection, on every
driver, so a walk advanced by a later call holds one row rather than a result set.

What is parked is what the result set described, what will end it, and the trace event the statement
is timed by — `crates/nvs-db/src/pg.rs`'s `PgCursor` is the shape, and each driver's is the same kind
of value: the column definitions and the wire's sequence position on MySQL and MariaDB, `COLMETADATA`
on SQL Server. The buffered members keep the same state beside a borrow of the connection, so one row
reader serves both paths per driver and they cannot disagree about what ends a stream.
`State::Streaming` is what refuses the second statement ([`core-classes/db-connection-busy-state`](/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state "Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset")),
read off a cell rather than off a lifetime.

**SQLite streams on one pinned thread per open walk.** `rusqlite`'s rows borrow the statement, which
borrows the connection, so there is no value to park; the statement is stepped on a thread from the
blocking pool and each row handed across instead. The thread is released when the walk is drained,
dropped, or its task ends — O(open streams), never O(requests served).

**Buffering is refused on every driver, in every disguise**, because it breaks the member's one
promise and makes a request's memory a function of a table's size. A driver whose protocol cannot be
advanced between calls throws a `RuntimeError` naming the driver and naming `query`; that refusal is
the fallback for a protocol that has no parked form, not a state any of the five drivers is in.

**An abandoned stream drains rather than poisons.** Each wire protocol frames its remaining rows
self-describingly to a terminating packet or `DONE` token, so the read back to a message boundary is
deterministic, bounded by the result set the caller asked for, and the connection returns to `Idle`
and to the pool. A read that *fails* mid-message is `Poisoned` and closed, as it is for every other
statement.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-streaming" title="A streaming result holds its connection until it is drained, and a second statement on it throws"><code>core-classes/db-streaming</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state" title="Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset"><code>core-classes/db-connection-busy-state</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/tests/db_stream.rs"><code>crates/nvs-stdlib/tests/db_stream.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-stream-as-answers-a-walk-and-not-a-buffered-result.nvst"><code>tests/conformance/core/db-stream-as-answers-a-walk-and-not-a-buffered-result.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-stream-on-sqlite-walks-its-rows-or-refuses-naming-the-driver.nvst"><code>tests/conformance/core/db-stream-on-sqlite-walks-its-rows-or-refuses-naming-the-driver.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-transactions">

## A transaction is a closure, and `Transaction` is a `Queryable` rather than a second query surface

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-transactions"><code>core-classes/db-transactions</code></a>
</div>

`$db->transaction($fn, {isolation?, readOnly?, retries?})` is the only transaction spelling. The
closure form is forced: with no destructors, an object-scoped transaction has no point at which to
roll back. A normal return commits, a throw rolls back and propagates, and a failed commit throws
`DbError`.

`Core\Db\Queryable` declares `query`, `queryAs`, `execute`, `executeMany`, `stream` and
`transaction`. `Connection` implements it and `Transaction implements Queryable by $connection`, so
the query surface is declared once and forwarded while the type system still expresses "this function
must run inside a transaction" and "this one does not care".

Two hazards a passable `Transaction` opens are closed explicitly. Using one after its `transaction()`
call returned throws. `$tx->rollBack($reason)` sets a rollback-only **flag** *and* throws
`Core\Db\RolledBack`, so an intervening `catch (Throwable)` cannot leave the transaction committed —
the owning frame acts on the flag, not on catching the signal.

Nesting on one connection issues `SAVEPOINT` and `ROLLBACK TO SAVEPOINT`, which removes the reason
`commit()`, `rollBack()` on the connection and `inTransaction()` each existed. `{retries: n}` re-runs
the closure on deadlock and serialization failure only, outermost transactions only, and defaults to
`0` because re-running a closure that sends mail is worse than surfacing the conflict.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>beginTransaction</code>/<code>commit</code>/<code>rollBack</code> on the connection and no <code>inTransaction</code>; nesting is a savepoint, and a rollback signal survives an intervening <code>catch (Throwable)</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-error" title="One DbError carries a normalised kind across every driver, and never a bound parameter"><code>core-classes/db-error</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-statement-members" title="Five members run a statement, results are buffered by default, and only one of them streams"><code>core-classes/db-statement-members</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0043.md">record 0043</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-transaction-answers-what-its-closure-answers.nvst"><code>tests/conformance/core/db-transaction-answers-what-its-closure-answers.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-transaction-hands-its-closure-a-transaction-and-not-a-connection.nvst"><code>tests/conformance/core/db-transaction-hands-its-closure-a-transaction-and-not-a-connection.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-sqlite-transaction-nests-as-a-savepoint.nvst"><code>tests/conformance/core/db-a-sqlite-transaction-nests-as-a-savepoint.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-roll-back-survives-a-catch-of-throwable.nvst"><code>tests/conformance/core/db-a-roll-back-survives-a-catch-of-throwable.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-roll-back-belongs-to-the-transaction-and-not-the-connection.nvst"><code>tests/conformance/core/db-roll-back-belongs-to-the-transaction-and-not-the-connection.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-error">

## One `DbError` carries a normalised kind across every driver, and never a bound parameter

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#db-error"><code>core-classes/db-error</code></a>
</div>

`Core\Db\DbError extends RuntimeError` and is readonly: `kind`, `sqlState`, `driverCode`,
`constraint`, `sql`. `ErrorKind` normalises the conditions applications actually branch on —
`UniqueViolation`, `ForeignKeyViolation`, `NotNullViolation`, `CheckViolation`, `Deadlock`,
`SerializationFailure`, `ConnectionLost`, `Timeout`, `Syntax`, `Permission`, `Other` — across four
drivers and five dialects, MariaDB carrying its own code table rather than MySQL's. SQLite's
`SQLITE_BUSY` and `SQLITE_LOCKED` map to `Deadlock`, so [`core-classes/db-transactions`](/docs/rules/core-classes/running-a-statement/#db-transactions "A transaction is a closure, and Transaction is a Queryable rather than a second query surface")'s retry
option works there too. The raw values stay available for what normalisation does not cover.

A class per condition was refused: it would add ten types to a deliberately small closed exception
set, and some boundaries are driver-dependent.

**Bound parameters never appear on the error, in the message, or in a trace** — a `Throwable` message
is a `secret` sink. The SQL text may, being developer-authored. The four raw values ride the throw on
one boxed slice, which is eight bytes narrower than the single slot it replaced and costs one
allocation on a path that is already allocating the object, the message and the backtrace.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An application branches on <code>ErrorKind::UniqueViolation</code> rather than on <code>1062</code>, <code>23505</code> or the text <code>&quot;Duplicate entry&quot;</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/running-a-statement/#db-transactions" title="A transaction is a closure, and Transaction is a Queryable rather than a second query surface"><code>core-classes/db-transactions</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#diagnostic-record" title="Every developer-facing output is one closed record"><code>errors/diagnostic-record</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-connect-refuses-a-block-it-cannot-read-by-the-field.nvst"><code>tests/conformance/core/db-connect-refuses-a-block-it-cannot-read-by-the-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-a-typed-row-throws-naming-the-column.nvst"><code>tests/conformance/core/db-a-typed-row-throws-naming-the-column.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-crate-boundary">

## The wire lives in `nvs-db` below the standard library, where a codec is borrowed and a state machine is written

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-crate-boundary"><code>core-classes/db-crate-boundary</code></a>
</div>

`crates/nvs-db` is one workspace crate below `nvs-stdlib`, depending on the runtime, the host's
stream and TLS, and configuration. `nvs-stdlib` depends on it and never the reverse: `Core\Db`'s
registry rows and reference cards stay where every other class's are, and the wire lives here, so the
graph stays a tree. A crate rather than a module, because five protocol implementations would
otherwise be rebuilt by every session that only touched `Core\Str`, and because the dependency set an
audit most wants to look at is worth having behind one manifest.

**No per-driver feature flags.** Every driver is in every binary, because `Core` means always
present and because the same binary should behave the same way everywhere — a feature matrix makes
"does this deployment speak MariaDB" a property of how someone built it. The cost is compile time and
binary size, spent to buy simplicity.

The rule the driver table encodes: **a codec is borrowed, a state machine is written.** Message
framing, value encoding and authentication mechanisms are large, fiddly and identical for everyone,
which is exactly what a sans-IO crate is. Sequencing — what to send next, what a park in the middle
means, when the connection is reusable — is where this project's own decisions live, and borrowing it
is what would have dragged an async runtime in. TDS has no such crate and is written by hand.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/connecting-to-a-database/#db-drivers-are-an-enum" title="The five drivers are an enum with one match per entry point, not a Driver trait"><code>core-classes/db-drivers-are-an-enum</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-busy-state" title="Busy state is a field on the connection, and a wire not at a message boundary is closed rather than reset"><code>core-classes/db-connection-busy-state</code></a> <a href="/docs/rules/core-classes/schemas/#schema-introspection" title="An existing database is read by catalog query, and no DDL parser exists at any tier"><code>core-classes/schema-introspection</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div></dl>

</div>
