---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Codecs, sessions, rate limits and signatures"
description: "One attribute generates a codec from the declared property list; sessions start explicitly; both rate-limit verbs are their own job."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/schemas/
  label: "Schemas and convergence"
next:
  link: /docs/rules/core-classes/uris-and-images/
  label: "URIs and images"
---

<p class="nv-section-lead">One attribute generates a codec from the declared property list; sessions start explicitly; both rate-limit verbs are their own job.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">12</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">10</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#derive-attribute">One written attribute per format generates a codec, and it is matched by resolved name</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#derive-field-list">The field list is the declared property list, and every field is a constructor parameter of the same name</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#derive-reports-every-field">A generated decoder reports every failed field at once, before the constructor runs</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#derive-generates-what-is-missing">The derive generates only the half the class does not write, at compile time, storing nothing per object</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#queue-storage-is-a-table">The job queue is two tables in a connection the operator names, converged by an explicit command</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#session-is-started-explicitly">A session is opened by calling <code>Core\Session::start</code>, and there is no ambient session array</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#ratelimit-two-members"><code>consume</code> and <code>shed</code> are two jobs with two verbs, and neither is a tier of the other</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#ratelimit-gcra">Both tiers run GCRA over one stored timestamp per key, and the <code>Decision</code> computes <code>retryAfter</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#ratelimit-unreachable-store-throws">An unreachable shared store throws, because whether to fail open is knowledge only the call site has</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#crypto-interop-tier"><code>Core\Crypto</code>'s algorithm roster is closed, every algorithm argument is required, and the sealed layout is what a browser reads</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#signature"><code>Core\Signature</code> signs a canonicalized payload, and the lifetime rides inside the signed bytes</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#router-signed-url"><code>Core\Router</code> signs a route name and its parameters, so a signature survives a remount</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="derive-attribute">

## One written attribute per format generates a codec, and it is matched by resolved name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#derive-attribute"><code>core-classes/derive-attribute</code></a>
</div>

A codec is generated only where a class asks for one, with one attribute per format: `#[Json\Derive]`
and `#[Db\Derive]` on the class, `#[Json\Field]` and `#[Db\Field]` on a property. The attribute makes
the class implement the corresponding interface; writing `implements Core\Json\Codec` beside it is
redundant but accepted.

**A compiler-recognized attribute is matched nominally.** The compiler acts on an attribute only when
its name *resolves*, through the ordinary namespace and `use` rules, to one of a closed `Core`-owned
list. So `#[Core\Json\Derive]` and a `use`d `#[Derive]` are one attribute reached two ways, while a
userland `type Derive = {};` is not it no matter how it is spelled, and a bare `#[{...}]` literal
never triggers one because it resolves to no name at all. An import binds a whole short name and is
never a namespace prefix, so `use Core\Json;` followed by `#[Json\Derive]` names nothing.

This is a carve-out of exactly one sentence, and it is the only one: attribute *retrieval* stays
structural, unchanged ([`attributes/structural-retrieval`](/docs/rules/attributes/#structural-retrieval "A retrieval matches a payload by the shape it satisfies, never by the name it was attached under")). Every future compiler-recognized
attribute joins the closed list; nothing else is ever matched by name. The list itself is a single
table in the compiler, which is what stops a per-record running total from going stale.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>No class is serialisable by being a class — a wire format is opted into at the declaration, and visibility never decides what is encoded</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list" title="The field list is the declared property list, and every field is a constructor parameter of the same name"><code>core-classes/derive-field-list</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-generates-what-is-missing" title="The derive generates only the half the class does not write, at compile time, storing nothing per object"><code>core-classes/derive-generates-what-is-missing</code></a> <a href="/docs/rules/attributes/#inert-metadata" title="An attribute is a shape literal on a declaration, and nothing is ever declared or instantiated for it"><code>attributes/inert-metadata</code></a> <a href="/docs/rules/attributes/#structural-retrieval" title="A retrieval matches a payload by the shape it satisfies, never by the name it was attached under"><code>attributes/structural-retrieval</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0062.md">record 0062</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-derive-is-a-recognized-attribute.nvst"><code>tests/conformance/core/db-derive-is-a-recognized-attribute.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-derive-encodes-declared-fields.nvst"><code>tests/conformance/core/json-derive-encodes-declared-fields.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-json-field-attribute-has-exactly-two-options.nvst"><code>tests/conformance/reject/a-json-field-attribute-has-exactly-two-options.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="derive-field-list">

## The field list is the declared property list, and every field is a constructor parameter of the same name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#derive-field-list"><code>core-classes/derive-field-list</code></a>
</div>

Fields are the class's own declared properties, in declaration order, private ones included.
Visibility is an access-control decision and has no business deciding a wire format; declaration
order fixes encode order, so output is byte-deterministic across runs and machines.

**Every non-skipped field must also be a constructor parameter of the same name and type**, or the
derive is a compile error at the attribute. For a class written with promoted parameters the two
lists are literally the same declaration, so this costs nothing. It is worth its cost because a
decode is an ordinary `new`: the generated decoder fills locals and calls the constructor, so a
decoded object is indistinguishable from a hand-built one and every invariant the constructor
establishes still holds. A `lateinit` property cannot be a field, being by definition not
constructor-assigned.

A field's type must be codec-reachable — a scalar, one of the named `Core` value types, an enum, an
inline shape, an `array<T>` or `?T` of one of those, or another class that itself has a codec.
Anything else is a compile error at the field. Recursion is fine and terminates on the data.

Two per-field overrides exist and no more: `name` renames one key or column, and `skip: true` removes
the field from the codec entirely. There is no whole-class naming policy — that would make a wire
format depend on a setting rather than on the source.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-attribute" title="One written attribute per format generates a codec, and it is matched by resolved name"><code>core-classes/derive-attribute</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-reports-every-field" title="A generated decoder reports every failed field at once, before the constructor runs"><code>core-classes/derive-reports-every-field</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-column-types" title="Every column has one natural Novis type, and the requested type converts losslessly or throws"><code>core-classes/db-column-types</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0022.md">record 0022</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0038.md">record 0038</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0030.md">record 0030</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-json-derive-field-needs-a-matching-constructor-parameter.nvst"><code>tests/conformance/reject/a-json-derive-field-needs-a-matching-constructor-parameter.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-derive-field-with-no-codec-is-refused-at-its-declaration.nvst"><code>tests/conformance/reject/a-derive-field-with-no-codec-is-refused-at-its-declaration.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst"><code>tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-db-derive-field-with-no-column-mapping-is-refused-at-its-declaration.nvst"><code>tests/conformance/reject/a-db-derive-field-with-no-column-mapping-is-refused-at-its-declaration.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst"><code>tests/conformance/core/json-decodes-a-nested-field-and-a-promoted-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="derive-reports-every-field">

## A generated decoder reports every failed field at once, before the constructor runs

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#derive-reports-every-field"><code>core-classes/derive-reports-every-field</code></a>
</div>

A generated decoder does not stop at the first bad field. It decodes every field, **accumulating
issues**, and if any were recorded it throws once, before the constructor is called, so no half-built
object exists.

An issue is `{path: string, message: string}`, and `path` is a dotted path into the payload —
`"address.city"`, `"tags.3"` — so a nested class's issues arrive at the top-level `catch` already
located. For a row decode it is the column name. `ParseError` carries the list; no new exception
class was added for it, because `ParseError` is already exactly the right node: *input did not match
a format this code declared*.

The accumulator is allocated only when the first issue is recorded, so the successful path — every
request that is not an error — allocates nothing for it.

One thing the record describes has not landed: `Core\Db\DbError` has no `issues` slot of its own, so
the row half's per-field refusals are reported on a `ParseError` instead. Gap 4 in
`crates/nvs-stdlib/src/db/mod.rs` is where that is recorded.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A decode failure is one throw carrying a located list, not the first problem the parser happened to hit</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list" title="The field list is the declared property list, and every field is a constructor parameter of the same name"><code>core-classes/derive-field-list</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-error" title="One DbError carries a normalised kind across every driver, and never a bound parameter"><code>core-classes/db-error</code></a> <a href="/docs/rules/errors/how-an-error-travels/#throwable-hierarchy" title="A limit report is not a Throwable, and the type checker knows it"><code>errors/throwable-hierarchy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst"><code>tests/conformance/core/json-derive-decodes-and-reports-every-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decodes-a-typed-list.nvst"><code>tests/conformance/core/json-decodes-a-typed-list.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="derive-generates-what-is-missing">

## The derive generates only the half the class does not write, at compile time, storing nothing per object

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#derive-generates-what-is-missing"><code>core-classes/derive-generates-what-is-missing</code></a>
</div>

The derive generates only what the class does not declare itself. A class writing its own encoder and
carrying the attribute gets the generated decoder and keeps its encoder — the common real case, since
a custom representation usually needs a mechanical inverse rather than a second bespoke one. A class
declaring **both** halves is a compile error: the attribute would generate nothing, and an attribute
with no effect is a mistake rather than a no-op.

`#[Db\Derive]` is one-directional. The row codec declares a read only; a write is an explicit
statement plus bound parameters, and generating an `INSERT` is the ORM already settled against. The
graph-copy operation gets no derive either: it handles every object with no per-class opt-in and is
not a declared wire contract at all. An anonymous shape encodes with no attribute, because a shape
literal has no declaration to carry one and no identity a property list could only guess at — its
encoding is structural, keyed on its field names alone. And there is **no validation**: a derived
codec checks types and presence, not that an email looks like one.

What it costs: a compile-time pass over the classes carrying the attribute, emitting straight-line
field-by-field code. **Nothing is stored per object and nothing is reflected at run time.** Footprint
is O(derived classes in compiled code), not O(objects) and not O(requests), and a program with no
derive attribute pays nothing at all, including no pass.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-attribute" title="One written attribute per format generates a codec, and it is matched by resolved name"><code>core-classes/derive-attribute</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list" title="The field list is the declared property list, and every field is a constructor parameter of the same name"><code>core-classes/derive-field-list</code></a> <a href="/docs/rules/types/objects-and-shapes/#object-literal" title="{name: value} builds an anonymous, methodless object and nothing more"><code>types/object-literal</code></a> <a href="/docs/rules/types/objects-and-shapes/#shape-type" title="{name: T} in type position is a structural shape checked by width subtyping, and {name?: T} marks a key that may be absent"><code>types/shape-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0029.md">record 0029</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-encodes-a-shape-as-an-object.nvst"><code>tests/conformance/core/json-encodes-a-shape-as-an-object.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decode-as-reads-only-a-class-that-declared-a-codec.nvst"><code>tests/conformance/core/json-decode-as-reads-only-a-class-that-declared-a-codec.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst"><code>tests/conformance/core/json-decode-as-admits-exactly-its-declared-type.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="queue-storage-is-a-table">

## The job queue is two tables in a connection the operator names, converged by an explicit command

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#queue-storage-is-a-table"><code>core-classes/queue-storage-is-a-table</code></a>
</div>

Durable background jobs live in one jobs table and one dead-letter table, in a connection the
operator names in `[queue]`. The runtime owns that schema **as a value** — a `Core\Db\Schema`
converged by `nvs queue migrate`, an explicit operator command — which is one description of the two
tables rather than one per dialect. A hand-written DDL list per backend is one chance to drift per
backend, and the backend nobody wrote is indistinguishable from one nobody supports.

**At most one *pending* job per dedupe key, and what enforces it is a plain column under a plain
unique key.** The jobs table carries `dedupe_pending`, which the statements maintain: it holds the
job's `dedupe_key` while the job is pending and `null` once it is claimed, succeeded, cancelled or
dead-lettered. A partial index (`… where state = 0`) and a stored generated column each say the same
thing, and neither is in [`core-classes/schema-is-a-value`](/docs/rules/core-classes/schemas/#schema-is-a-value "A schema is a value with three spellings, and its array form is the canonical one")'s vocabulary — they are two dialects'
answers to one requirement, which is exactly what a schema value exists to stop being. A null
collides with nothing on any of the five, because a unique key's nulls are distinct on every backend
([`core-classes/a-unique-key-reads-nulls-as-distinct`](/docs/rules/core-classes/schemas/#a-unique-key-reads-nulls-as-distinct "A unique key reads nulls as distinct on every backend, and SQL Server spells that as a filtered index")) — four give it directly and SQL Server
through the filtered index its emitter writes — so the guarantee is one guarantee, stated once, on
every backend `Core\Queue` runs a statement against.

That is the queue reading a property of the vocabulary rather than the queue asking for one. The
alternative — a `not null` column with a generated token per released row — is refused for a reason
that outlives any one backend: such a column cannot be added to a table that already holds rows, so
it would be a schema no existing deployment could converge to, while a nullable one arrives as a
`Safe` step.

DDL is an injection sink and a privileged act, so **the runtime never issues it implicitly**, not at
boot and not from a request; applying the plan takes `db.schema` like any other DDL
([`core-classes/schema-apply-capability`](/docs/rules/core-classes/schemas/#schema-apply-capability "Planning is an ordinary read, and applying takes db.schema under a member named for its risk")).

It may be the application's own database, and that is the recommended configuration, because a
transactional enqueue — the property the whole design rests on — requires it. A separate queue
database is permitted and silently gives up that property, which is why the documentation says so at
the point the option is offered.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no broker, no daemon of its own and no implicit boot-time DDL — the tables are created by <code>nvs queue migrate</code> and by nothing else</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/schemas/#schema-converges" title="A plan is the difference between a schema value and a live database, and Core knows no migrations"><code>core-classes/schema-converges</code></a> <a href="/docs/rules/core-classes/schemas/#schema-apply-capability" title="Planning is an ordinary read, and applying takes db.schema under a member named for its risk"><code>core-classes/schema-apply-capability</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0084.md">record 0084</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0145.md">record 0145</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0153.md">record 0153</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0187.md">record 0187</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-push-refuses-an-enqueue-with-no-queue-configured.nvst"><code>tests/conformance/core/queue-push-refuses-an-enqueue-with-no-queue-configured.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/queue-push-judges-its-options-before-it-opens-anything.nvst"><code>tests/conformance/core/queue-push-judges-its-options-before-it-opens-anything.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="session-is-started-explicitly">

## A session is opened by calling `Core\Session::start`, and there is no ambient session array

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#session-is-started-explicitly"><code>core-classes/session-is-started-explicitly</code></a>
</div>

There is no ambient session array and no implicit session start. A script calls `Core\Session::start`
— or an equivalent resuming a presented identifier — before reading or writing session state, so
"this request uses sessions" is a line in the source rather than a fact discoverable only by grepping
for a superglobal. A member called before `start` throws naming itself, and the refusal is catchable
at the root like any other.

The shape is fixed here: a class, an explicit start, no ambient array. The mechanics are a feature of
their own — the store is the shared cache tier or the database, the local tier is refused at boot
because a session in per-core memory is not a session, there is no lock, and expiry is the store's
own rather than a sweeper's.

What this costs is one line per request that uses sessions. What it buys is that a request that does
not use them pays nothing, which the ambient version could never promise.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$_SESSION</code> does not exist, nothing starts a session implicitly, and a member called before <code>start</code> throws</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a> <a href="/docs/rules/core-classes/processes-and-files/#cli-arguments" title="Core\Cli::arguments is how a program reads the words it was started with, at every depth"><code>core-classes/cli-arguments</code></a> <a href="/docs/rules/core-classes/processes-and-files/#script-args" title="Core\Script::args is the value the current isolate was spawned with, and null where there was none"><code>core-classes/script-args</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0008.md">record 0008</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-a-member-called-before-start-throws-naming-it.nvst"><code>tests/conformance/core/session-a-member-called-before-start-throws-naming-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-every-member-refuses-a-record-nobody-opened.nvst"><code>tests/conformance/core/session-every-member-refuses-a-record-nobody-opened.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-takes-the-presented-identifier-or-none-at-all.nvst"><code>tests/conformance/core/session-start-takes-the-presented-identifier-or-none-at-all.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-refuses-a-tree-that-configured-no-store.nvst"><code>tests/conformance/core/session-start-refuses-a-tree-that-configured-no-store.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ratelimit-two-members">

## `consume` and `shed` are two jobs with two verbs, and neither is a tier of the other

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#ratelimit-two-members"><code>core-classes/ratelimit-two-members</code></a>
</div>

`Core\RateLimit::consume` and `::shed` are two jobs with two verbs, and the verb says which one you
are doing. `consume` enforces a policy the application promised somebody — a plan quota, a login
limit — over the shared store, coherently, because a policy only approximately enforced is not
enforced. `shed` drops load to keep a host up, per core, and an approximate answer is entirely
adequate for that because the goal is "less than the amount that hurts", not a number. `shed`'s
contract states its arithmetic out loud: its count is per core, so a limit of 100 across eight cores
admits up to 800.

They are not two tiers of one operation, and a flag on one member could not carry the difference.

Four things are deliberately absent. **Edge and flood limiting** — no per-IP, per-path or connection
limit as a deployment feature: a proxy owns it earlier and cheaper, and doing it here means paying
for the request in order to reject it. **All configuration** — a limit is application policy, so
there is no `[ratelimit]` block at all. **Any automatic enforcement** — the member returns a decision
and writes no `429`. **Distributed reservation** and multi-key atomic checks, which are considerably
more machinery with no named use case yet.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no per-IP or per-path limiting, no <code>[ratelimit]</code> config block and no middleware writing a <code>429</code> for you</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-gcra" title="Both tiers run GCRA over one stored timestamp per key, and the Decision computes retryAfter"><code>core-classes/ratelimit-gcra</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-unreachable-store-throws" title="An unreachable shared store throws, because whether to fail open is knowledge only the call site has"><code>core-classes/ratelimit-unreachable-store-throws</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#topic" title="Core\Topic is the only way two connections meet, and a slow subscriber is closed rather than tolerated"><code>core-classes/topic</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0075.md">record 0075</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-shed-is-per-core-and-reaches-no-store.nvst"><code>tests/conformance/core/ratelimit-shed-is-per-core-and-reaches-no-store.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-shed-keys-its-own-memory.nvst"><code>tests/conformance/core/ratelimit-shed-keys-its-own-memory.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-weighs-a-call-with-one-trailing-shape.nvst"><code>tests/conformance/core/ratelimit-weighs-a-call-with-one-trailing-shape.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ratelimit-gcra">

## Both tiers run GCRA over one stored timestamp per key, and the `Decision` computes `retryAfter`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#ratelimit-gcra"><code>core-classes/ratelimit-gcra</code></a>
</div>

Both members implement the **generic cell rate algorithm** — a leaky bucket expressed as one stored
timestamp, the theoretical arrival time of the next permitted request. `limit` per `per` sets the
drain rate, `burst` (defaulting to `limit`) sets how much may arrive at once, and `cost` weights one
call so an expensive endpoint may consume five units of the same quota.

One duration of arithmetic per call, and **one stored timestamp per key** rather than a window of
them. In the shared store that is the difference between O(1) and O(requests in window) memory per
key, which is what makes limiting per user affordable at a million users. `retryAfter` is computed
rather than estimated, where a sliding-window counter can only answer "sometime in the next window" —
the answer that makes clients retry in a burst at the window edge. Both tiers run the identical
algorithm, so moving a call between them changes the *guarantee* and not the shape of the answer.

The `Decision` is readonly `allowed`, `limit`, `remaining` and `retryAfter`, the last being `?Duration`
and `null` exactly when `allowed` is true — absence is `?T`, not a sentinel zero. Both members are
qualifier-neutral. The key **accepts `tainted`**, since a tenant or account id is user-derived by
nature and the store's protocol is length-prefixed, so a key cannot reshape a command; it
**refuses `secret`**, because using a signing key as a rate-limit key writes it into a store with a
TTL. The in-process tier is an existing crate; the shared tier is a small script of our own, an
algorithm over our own state.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-unreachable-store-throws" title="An unreachable shared store throws, because whether to fail open is knowledge only the call site has"><code>core-classes/ratelimit-unreachable-store-throws</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a> <a href="/docs/rules/types/text-and-literal-types/#duration-literal" title="1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml"><code>types/duration-literal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0075.md">record 0075</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-decision-readers-agree-with-the-sweep.nvst"><code>tests/conformance/core/ratelimit-decision-readers-agree-with-the-sweep.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-shed-weighs-a-call-and-bounds-a-burst.nvst"><code>tests/conformance/core/ratelimit-shed-weighs-a-call-and-bounds-a-burst.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-shed-bounds-its-window-on-both-sides.nvst"><code>tests/conformance/core/ratelimit-shed-bounds-its-window-on-both-sides.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-refuses-a-limit-that-admits-nothing.nvst"><code>tests/conformance/core/ratelimit-refuses-a-limit-that-admits-nothing.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ratelimit-unreachable-store-throws">

## An unreachable shared store throws, because whether to fail open is knowledge only the call site has

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#ratelimit-unreachable-store-throws"><code>core-classes/ratelimit-unreachable-store-throws</code></a>
</div>

`consume` on an unreachable shared store throws. It does **not** return `allowed: true`.

This is the load-bearing choice. A security control that fails open silently is worse than no
control, because the deployment believes it has one — and whether *this* limiter should fail open or
closed is knowledge only the call site has. A login throttle must fail closed; a plan quota should
fail open rather than take the product down. Throwing puts the decision where the knowledge is, and
an application that chooses to fail open does so in a `catch` that a reader can see and a reviewer
can question.

`shed` cannot fail this way, its state being in the core's own memory, which is one more reason the
two are different members rather than one with a flag.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-gcra" title="Both tiers run GCRA over one stored timestamp per key, and the Decision computes retryAfter"><code>core-classes/ratelimit-gcra</code></a> <a href="/docs/rules/errors/how-an-error-travels/#propagation" title="An error propagates as a checked return, never by unwinding"><code>errors/propagation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0075.md">record 0075</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0002.md">record 0002</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/ratelimit-refuses-rather-than-deciding-allowed.nvst"><code>tests/conformance/core/ratelimit-refuses-rather-than-deciding-allowed.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="crypto-interop-tier">

## `Core\Crypto`'s algorithm roster is closed, every algorithm argument is required, and the sealed layout is what a browser reads

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#crypto-interop-tier"><code>core-classes/crypto-interop-tier</code></a>
</div>

`Core\Crypto`'s algorithm roster is closed, every algorithm argument is required, and no member name
carries an algorithm.

The roster is AES-256-GCM beside XChaCha20-Poly1305, PBKDF2-HMAC-SHA256, HKDF-SHA256, ECDH over P-256
and X25519, and four signature algorithms — RSASSA-PKCS1-v1_5 and RSASSA-PSS over SHA-256, ECDSA over
P-256, and Ed25519. AES Key Wrap and the Concat KDF are internal to PBES2 and ECDH-ES and never members.
Out, permanently: AES-CBC, AES-CTR, AES-128, RSA encryption in any form, RSA key generation, P-384,
ES384, RS384, RS512, PS384, PS512, ES256K, and JWE's `A*CBC-HS*` content encryption. A closed roster that
is only a habit reopens at the first ticket, so the list is written down.

**Where two algorithms take the same parameters they are one member with a closed enum argument**, the
way `Core\Hash::of` takes a `Core\Digest`; where the parameters differ they are separate members. **No
algorithm argument has a default** — not a cipher, not a digest, not a curve. A default is how a call
site copied from another file keeps an algorithm nobody re-read, and making the argument required buys a
`grep` that finds every use of a primitive on the day it has to be retired. The one member answering a
key with no algorithm in it, `generateKey()`, takes no algorithm argument at all: its 32 octets are the
single length both ciphers and every roster protocol share.

**AES-256-GCM is on the roster for interoperability and its sealed bytes say so**: `nonce(12) ‖
ciphertext ‖ tag(16)`, which is exactly what WebCrypto's `encrypt` answers with its IV put in front, so
a browser splits at byte 12 and does nothing else. XChaCha's stay `nonce(24) ‖ ciphertext ‖ tag(16)` and
remain the pair to prefer when both ends are Novis — advice on the member's reference card, never a
default. The nonce is drawn by the member and there is no nonce parameter, so AES-GCM carries its
birthday bound around 2^32 messages under one key; that bound is the price of reading what a browser
wrote, and the answer when a program approaches it is the other cipher rather than a counter the caller
keeps.

**A key is validated where it is read, not where it is used.** Every public key is checked at `read` — on
the curve for P-256, inside 2048–8192 bits for RSA — and an all-zero X25519 shared secret is refused at
`agree`, so there is no later site at which the check could be forgotten. An RSA key's scheme is fixed
when it is read, which is [`security/algorithm-comes-from-the-key`](/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key "The algorithm comes from the key and never from the token") held for the one key type two JWS
algorithms share, and an RSA pair is never generated. A key off the wire that fails is a `RuntimeError`;
a malformed key the program built is a `LogicError`.

The interop claim is checkable rather than intended: `tools/webcrypto-vectors.mjs` freezes WebCrypto's
own output for every algorithm here, derived from labels so a rerun writes the same bytes.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no cipher name in a string and no unauthenticated mode to reach for, so <code>aes-256-ecb</code> has no spelling</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/what-belongs-in-core/#tier-roster" title="Every subsystem's tier is recorded once in the roster, including the ones no milestone has built"><code>core-api/tier-roster</code></a> <a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#jwe-compact-subset" title="Core\Jwe speaks compact JWE with A256GCM alone, and the static that built the key picks the algorithm"><code>security/jwe-compact-subset</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0179.md">record 0179</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/crypto-aes-gcm-sealed-bytes-are-nonce-then-ciphertext-then-tag.nvst"><code>tests/conformance/core/crypto-aes-gcm-sealed-bytes-are-nonce-then-ciphertext-then-tag.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/crypto-seals-and-opens-under-the-cipher-the-call-names.nvst"><code>tests/conformance/core/crypto-seals-and-opens-under-the-cipher-the-call-names.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/crypto-both-derivations-repeat-exactly-and-separate-on-every-argument.nvst"><code>tests/conformance/core/crypto-both-derivations-repeat-exactly-and-separate-on-every-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/crypto-public-key-round-trips-every-kind-through-every-encoding.nvst"><code>tests/conformance/core/crypto-public-key-round-trips-every-kind-through-every-encoding.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/crypto-sign-writes-what-webcrypto-signed-and-verify-takes-every-one.nvst"><code>tests/conformance/core/crypto-sign-writes-what-webcrypto-signed-and-verify-takes-every-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="signature">

## `Core\Signature` signs a canonicalized payload, and the lifetime rides inside the signed bytes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#signature"><code>core-classes/signature</code></a>
</div>

`Core\Signature::sign` takes a map and an options bag carrying a key ring and an optional expiry, and
answers a token; `verify` takes a token and a key ring and answers the payload or throws.

**The input is a map, never text.** `sign` canonicalizes the payload itself — keys sorted, each value
encoded with its type — so there is no assembled string for the two sides to disagree about, which is
what every hand-rolled signing helper gets wrong. **The lifetime is inside the signed bytes**, not
beside them, so a holder cannot edit it and there is no second parameter to keep in step. The token
is URL-safe by construction — unpadded URL-safe base64 — so every octet is a legal cookie octet and a
legal query-string value and nothing downstream escapes it again. The key ring is newest-first:
signing uses the head, verifying tries the ring in order, rotating is prepending and retiring is
dropping the tail. One rotation vocabulary in the language, not two.

**A verified payload is `tainted`.** A token minted by one service and read by another is not the
signed-cookie case, where the plaintext was the application's own when it went in; "we authored this
payload" is not a property the checker can see, so it is not one the return type may assume.

**Not shipped.** There is no `Core\Signature` in `crates/nvs-stdlib/src/`; the nearest landed member
is `signed_cookie.rs`, whose key-ring shape this one adopts.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no string to assemble and therefore nothing for two sides to disagree about, and a verified payload comes back <code>tainted</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#router-signed-url" title="Core\Router signs a route name and its parameters, so a signature survives a remount"><code>core-classes/router-signed-url</code></a> <a href="/docs/rules/core-classes/uris-and-images/#uri-removable-components" title="Core\Uri spends the omitted-versus-null distinction on three components and on one query parameter at a time"><code>core-classes/uri-removable-components</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a></dd></div></dl>

</div>

<div class="nv-rule" id="router-signed-url">

## `Core\Router` signs a route name and its parameters, so a signature survives a remount

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#router-signed-url"><code>core-classes/router-signed-url</code></a>
</div>

`Core\Router` keeps its own signing pair — one that mints a signed URL from a route name and its
typed parameters, and one that verifies against the match the server already made. Parsing a built
URL and signing it would reach most of this, and convenience alone would not justify two more
members.

**One property does.** A compiled route table may be served at one mount prefix or another, so a
signed *path* stops verifying the moment a mount moves. Signing the route **name** and its parameters
survives a remount, and verification reads the existing match rather than re-parsing anything.

**Verification is the application's, called by hand, wherever it keeps it.** Nothing verifies a
signature for you, because the program that renders the refusal is the program that should decide
when to ask.

**Not shipped.** `crates/nvs-stdlib/src/router.rs` carries neither member, and
[`core-classes/signature`](/docs/rules/core-classes/codecs-sessions-and-signatures/#signature "Core\Signature signs a canonicalized payload, and the lifetime rides inside the signed bytes") — the payload half both would sign — is not built either.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#signature" title="Core\Signature signs a canonicalized payload, and the lifetime rides inside the signed bytes"><code>core-classes/signature</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a></dd></div></dl>

</div>
