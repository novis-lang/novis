# ADR 0024 — Untrusted input is a distinct type; injection sinks demand laundering

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** a `tainted` compile-time qualifier on `string`/`bytes`; how it enters, propagates, and is
  removed; the sinks that refuse a tainted value (HTML output, SQL query text, process arguments, HTTP
  header values, filesystem paths); the `Core\Html\Markup` safe-markup type and the HTML output sink's
  auto-escape default.
- **Amends:** [0007](0007-explicit-type-system.md) § 2 — adds a `tainted` qualifier axis to the conversion
  table for `string`/`bytes`, following the same total/checked shape as every other conversion; every other
  row is unchanged.
  [0009](0009-string-and-bytes.md) § 4 — the scalar payload pulled out of `mixed` may now also be
  `tainted string`/`tainted bytes`; § 3's `bytes`/`string` conversion is unaffected and preserves the
  qualifier across either direction.
  [0012](0012-no-superglobals.md) — every method on `Core\Request`/`Core\Server`/`Core\Session`/`Core\Env`/
  `Core\Cli`/`Core\Script::args()` returns the `tainted` variant of whatever it already returned; the
  mapping table and the method-signature deferral are otherwise unchanged.
- **Relates to:** [0004](0004-memory-for-simplicity.md) (the qualifier is compile-time-only and erased
  before codegen — security bought without spending memory or a runtime representation),
  [0011](0011-functions-and-constants-are-class-members.md) (the laundering functions are ordinary `Core`
  static methods, not new syntax), [0013](0013-comparable-interface.md) and
  [0014](0014-property-observer.md) (precedent for "a declared mechanism, not an ambient one" and "closed,
  non-hookable" reasoning), [0019](0019-reflection-and-ast-parsing-are-core-features.md) (precedent for
  "a security-relevant analysis belongs in the compiler itself, not an aftermarket tool"),
  [0020](0020-error-escalation-ladder.md) (log injection is already closed by that ADR's structured
  JSON-Lines writer — see § 4), CLAUDE.md's priority ordering (security ranks above simplicity, which is
  the explicit justification for § 5's one deliberate exception to "no ambient behavior")

> **In short:** [ADR 0012](0012-no-superglobals.md) already funnels every piece of untrusted input through
> five `Core` accessor classes — unlike PHP, where untrusted data can enter through dozens of implicit
> paths, MWL already knows exactly where it comes from. This ADR spends that fact: a `string`/`bytes`
> returned by one of those classes (or by anything else that hands a script data it did not itself just
> compute — see *Context*) carries a `tainted` qualifier, erased before codegen, that behaves like any other
> checked type distinction in [ADR 0007](0007-explicit-type-system.md) — no implicit conversion out of it,
> concatenation/interpolation poisons the result, and a handful of `Core` sinks (HTML output, SQL query
> text, process arguments, HTTP headers, filesystem paths) refuse it outright. Laundering happens only
> through narrow, sink-named `Core` functions whose return type is provably safe for that one sink, plus one
> loud escape hatch (`Core\Taint::assertTrusted`) modeled on this project's own `unsafe` policy. HTML output
> gets one deliberate exception to "nothing happens implicitly": the response sink auto-escapes any
> non-`Markup` value by default, and only a source-literal string converted `as Markup` (or a composition of
> `Markup` values) bypasses that — the one place this project spends priority-4 simplicity to buy priority-1
> security, and says so.

## Context

[ADR 0012](0012-no-superglobals.md) closed every ambient PHP superglobal to a `static` method call on one
of five reserved classes. That ADR's argument was traceability — "a reviewer sees exactly which `Core`
class, and therefore which trust boundary, a line depends on" — but the type system stops at the call site.
Once `Core\Request::query('id')` returns a plain `string`, nothing distinguishes it from a literal written
in the source two lines above. Concatenating the two into a SQL string or an HTML fragment is invisible to
`mwl check` — the exact gap that makes XSS and SQL injection the two userland failure modes the project set
out to close, per the brainstorming that opened this decision.

Three prior attempts at this general idea are worth naming, because MWL's answer differs from each for a
reason specific to this codebase:

- **Perl's taint mode** flags data at runtime and checks it at each dangerous call. It works, but it is a
  runtime tag on every scalar, checked on every use — exactly the representation and per-operation cost
  [ADR 0004](0004-memory-for-simplicity.md) argues against paying for when a free alternative exists, and it
  buys nothing MWL's ahead-of-time type checker cannot prove instead.
- **Google's safe-html-types / Error Prone `@CompileTimeConstant`** bolt static analysis onto Java from
  outside the compiler, as an optional, skippable build step. [ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) already made this project's stance on that shape of choice explicit for reflection and AST
  parsing: a security-relevant analysis that lives outside the compiler is optional by construction. The
  same reasoning applies here.
- **Go's `html/template`** solves the HTML half by parsing the template and auto-escaping by context — real
  and effective, but it is a property of one template engine, not of the language's type system, so it says
  nothing about SQL, shell commands, or file paths. MWL folds the same auto-escaping idea into § 5 below, but
  as one instance of a general type-system mechanism rather than the whole answer.

**Why the source list is a rule, not a fixed enumeration.** A value read back from anything that persists
across requests carries exactly the same risk as one read live from the request: if a request handler ever
stored attacker-influenced content (directly, or via a bug that assumed something was safe when it wasn't),
reading it back and rendering it is second-order/stored injection — the same vulnerability class, one hop
later, and arguably more dangerous because the developer reading it back has usually forgotten it can be
attacker-shaped at all. `Core\Session` already exists among ADR 0012's five classes; `Core\Db` result rows
and `Core\Cache` reads will exist by M8/M9 and are not designed yet. Rather than re-litigating taint sources
in a new ADR each time one of those lands, this ADR fixes the standing rule — *taint enters wherever a
script receives data it did not just compute: a live request, a persisted store, another process, or the
environment* — and leaves each concrete class's exact return types to whichever milestone designs it, the
same deferral [ADR 0012](0012-no-superglobals.md) already used for method signatures.

## Decision

### 1. `tainted` is a compile-time qualifier on `string`/`bytes`, erased before codegen

`tainted string` and `tainted bytes` join the type grammar as a qualified form of the two scalar types
[ADR 0009](0009-string-and-bytes.md) already defines — not a class, not a wrapper, not a runtime tag. It is
checked exactly once, by `mwl check`, and carries no representation at all past that point: no extra byte in
the value's header, no refcount change, no cost on the hot path. This is security bought for free under
[ADR 0004](0004-memory-for-simplicity.md)'s ordering, the same way a `readonly` property costs nothing once
compiled.

Every method on `Core\Request`, `Core\Server` (header values and any other client-influenced field —
`REQUEST_METHOD` from a fixed enum-shaped set is not attacker-shaped the same way and is not required to be
tainted), `Core\Session`, `Core\Env`, `Core\Cli`, and `Core\Script::args()` returns the tainted form of
whatever it already returned. Structured input stays `array<mixed>` exactly as
[ADR 0007](0007-explicit-type-system.md) § 6 and [ADR 0009](0009-string-and-bytes.md) § 4 already decided —
this ADR is about the scalar payload once it is pulled out of `mixed`, the same framing those two ADRs
already used.

### 2. Propagation: tainted poisons; a checked conversion launders for free

Any operation combining a tainted operand with an untainted one — concatenation, interpolation, a string
function, an array of scalars — produces a tainted result. This is the same "poisoned" shape
[ADR 0007](0007-explicit-type-system.md) already uses for mixed-type arithmetic, applied to a new axis.

A checked `as` conversion to a type that already throws on a malformed shape — `as uint`, `as int`,
`as float`, `as bool`, `as` an enum's backing type — **removes the qualifier on success**, no new syntax
needed: `Core\Request::query('id') as uint` already throws on `"abc"`, `"-1"`, or `""`
([ADR 0012](0012-no-superglobals.md)'s own example), and a value that survives that check has had its shape
proven, which is what laundering means for a non-string type. `bytes as string` and `string as bytes`
([ADR 0009](0009-string-and-bytes.md) § 3) preserve the qualifier across either direction — UTF-8 validity
says nothing about whether the content is safe for a given sink.

### 3. Laundering functions are narrow, sink-named, and never generic

The only way to remove a `tainted` qualifier from a `string`/`bytes` value (short of the conversions in
§ 2) is a `Core` function whose return type is the plain, unqualified type and whose contract states which
one sink it is safe for — `Core\Html::escape(tainted string): string` for HTML text,
`Core\Db::quoteIdentifier(tainted string): string` for a dynamic table/column name, and others as each
sink's stdlib class is designed (illustrative, not fixed — see *Revisiting*). There is deliberately no
generic `sanitize()` or `clean()`: a value safe for HTML text is not safe for a shell argument, and a single
catch-all invites exactly the false confidence this ADR exists to prevent.

One narrow escape hatch exists for the case no built-in launderer fits — the developer has validated the
value themselves and needs to say so: `Core\Taint::assertTrusted(tainted string, string $reason): string`.
This is modeled directly on this project's own `unsafe` policy (CLAUDE.md: forbidden workspace-wide, opt
down to `deny` with named, reasoned allows) — forbidden by default, an escape hatch that is rare, greppable,
and carries a written reason at the call site, never a silent cast.

### 4. Sinks that refuse a tainted value

- **HTML/text output** — see § 5; the mechanism there makes an explicit launderer unnecessary for ordinary
  text.
- **`Core\Db`'s query-text parameter** requires the plain, unqualified `string` — a tainted value cannot
  reach it without an explicit launderer first, which is exactly the nudge toward binding instead of
  concatenating. The **bound-parameters argument stays `array<mixed>`, tainted-friendly by design**: binding
  is the mechanism that makes an arbitrarily tainted value safe, so requiring laundering there would be
  pure friction with no security benefit. Dynamic identifiers (table/column names, which SQL cannot
  parameterize) go through `Core\Db::quoteIdentifier()` or an allowlist check instead.
- **`Core\Process`'s command execution** (M8+, behind the capability gate already named in the plan) takes
  an executable path and an argv array, each element requiring the plain type — and, more fundamentally,
  there is no shell-interpolation form at all. An argv array with no shell in between removes the escaping
  question rather than answering it, which is a stronger guarantee than any amount of quoting.
- **`Core\Http`'s header-value setter** requires the plain type (header/response-splitting injection).
- **A `Core\Fs` call taking a caller-influenced path component** requires the plain type; the real launderer
  (reject path separators and `..`, or resolve-and-verify against a base directory) is stdlib design due at
  the milestone that builds it, per the same deferral as § 3's roster.

**`Core\Log` is deliberately not in this list.** [ADR 0020](0020-error-escalation-ladder.md) already fixed
`Core\Log`'s writer as a JSON-Lines serializer — a structured field is escaped by the serializer, never by
string concatenation into a line — which already closes log-forging/injection independently of this ADR.
Logging tainted content is *desired*, not a risk: recording exactly what an attacker sent is the point of a
security log. Stated here so a future reader does not go looking for a redundant fix.

### 5. HTML: auto-escape by default, `Core\Html\Markup` the only raw-write bypass

This is the one deliberate exception to this project's otherwise-consistent stance that nothing happens by
position, only by declaration ([ADR 0008](0008-static-and-global.md), [ADR 0012](0012-no-superglobals.md),
[ADR 0013](0013-comparable-interface.md), [ADR 0014](0014-property-observer.md)). CLAUDE.md's priority
ordering ranks security above simplicity for exactly this kind of conflict, and an omitted escape call is
the single most common real-world XSS root cause — so this ADR spends that priority explicitly rather than
holding the "no magic" line for its own sake.

`Core\Html\Markup` is a small value type, peer to `string` the way [ADR 0009](0009-string-and-bytes.md)'s
`bytes` is peer to `string`, representing HTML known to be safe to write raw:

- A **source-literal string**, converted with `as Markup`, is trusted — it is exactly what the developer
  wrote in the file, the same trust level any other literal already carries. A runtime-computed or
  `tainted` string can **never** become `Markup` via `as`; only a literal token qualifies, which closes the
  obvious bypass ("compute the escape-defeating payload at runtime, then cast it").
- `Markup` **+** `Markup` is `Markup` — composing trusted fragments (what templating already does) stays
  cheap and stays trusted.
- Interpolating any **non-`Markup`** value — tainted or not — into a `Markup`-building position (the
  existing inline-HTML `<?= expr ?>` slot from M1's dual-mode lexer, or a future templating helper)
  auto-escapes it via `Core\Html::escape()` and lifts the result to `Markup`. This is the sink's *only*
  behavior: it never distinguishes tainted from untainted, because escaping already neutralizes either one
  structurally.
- The HTTP response-write sink accepts only `Markup`. A developer never manually calls an escape function
  for ordinary text interpolation — only hand-composing a raw markup fragment reaches for `Markup`/
  `as Markup` on a literal.

## Consequences

**Positive**

- One type-system feature — a qualifier axis on two already-existing primitives — closes several
  vulnerability classes at the exact call sites developers already write, for zero runtime cost.
- Second-order/stored injection is closed by the same standing rule (§ *Context*), not left as a gap
  discovered after `Core\Db` ships.
- Building a SQL query or an HTML fragment directly from live input becomes a compile error, not a
  code-review habit or a linter suggestion that can be silenced.
- Log injection is confirmed already closed by [ADR 0020](0020-error-escalation-ladder.md) rather than given
  a second, possibly-inconsistent fix here.

**Negative**

- A genuinely new type-checker feature: M2's type checker must implement the qualifier axis, its poisoning
  propagation, and the checked-conversion laundering rule before M7's `Core\Request` can return anything
  meaningful — a real sequencing dependency, not just an API addition.
- **False positives are the accepted failure direction.** A value the developer knows is safe (config data
  the app itself wrote to its own session) can still be marked tainted once it round-trips through a
  persisted store this ADR treats conservatively; `Core\Taint::assertTrusted` exists for exactly this, at
  the cost of one written reason per call site.
- **`mwl convert` gains a real, non-mechanical gap**, in the family [ADR 0009](0009-string-and-bytes.md) and
  [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already carry: a ported PHP page that
  deliberately echoed raw HTML built from a variable (a common templating pattern) now gets an implicit
  escape it did not have before — a behavior change, not a syntax rewrite, and it needs a human to add
  `as Markup` or a `Markup`-returning helper rather than a mechanical conversion.

## Alternatives rejected

- **Runtime-only taint tracking (Perl's model).** Rejected in *Context*: throws away the one advantage a
  static type checker gives over a dynamic tag, and pays a representation and per-operation cost
  [ADR 0004](0004-memory-for-simplicity.md) argues against when the compile-time version is free.
- **A bolt-on static-analysis pass outside `mwl check`**, mirroring Error Prone. Rejected in *Context* on
  the same grounds [ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) already used for
  reflection: an optional, skippable analysis is not the same guarantee as a compiler that refuses to emit
  code, and this project has already decided that split once.
- **Require an explicit escape call at every HTML interpolation site, no auto-escape default.** This is the
  option § 5 argues past: rejected because CLAUDE.md's own priority ordering puts security above simplicity,
  and "the compiler escapes for you unless you opt out with a literal" is strictly safer than "a human
  remembers, every time" — the single most common real-world XSS root cause is exactly the omitted call this
  removes structurally.
- **Require `Core\Db`'s query-text parameter to be a compile-time literal, not merely untainted** (mirroring
  the stricter option considered for `Markup`). Rejected for now: it would block legitimate dynamic query
  assembly (pagination, a sort column chosen from an allowlist) that already has to go through an
  identifier-quoting helper regardless — a strictly stronger guarantee purchased for a real ergonomics loss
  that the untainted-only rule does not need to pay. Revisit per *Revisiting* if practice shows the weaker
  rule insufficient.

## Revisiting

- **If real MWL programs show the untainted-string SQL rule insufficient** — a class of query-building bugs
  slips through because an "untainted" string was easy to construct without real validation — reconsider the
  literal-only stricter mode rejected above.
- **Whether `Core\Session`, and later `Core\Db` result rows and `Core\Cache` reads, should expose a
  narrower, provably-safe subtype instead of blanket `tainted`** is an open stdlib question, deferred to
  whichever milestone designs each class's real API — the same deferral
  [ADR 0012](0012-no-superglobals.md) already used for `Core\Request`'s exact method signatures.
- **The exact laundering-function roster** (`Core\Html::escape`, a matching attribute-context escaper,
  `Core\Db::quoteIdentifier`, `Core\Taint::assertTrusted`, and whatever `Core\Process`/`Core\Fs` need) is
  stdlib design due at M8, illustrative only here, the same status
  [ADR 0011](0011-functions-and-constants-are-class-members.md)'s *Revisiting* already gives every `Core`
  class roster.
- **Raw/unparsed request-body access** (a JSON payload, a webhook body, an arbitrary content-type) is not
  yet named among `Core\Request`'s planned methods — [the plan](../implementation-plan.md)'s M7 paragraph
  currently lists only multipart and urlencoded body parsing. Flagged here as a known gap for whoever
  designs `Core\Request`'s real surface at M7; this ADR does not resolve it, and only fixes that whatever
  that method turns out to be, it returns `tainted` like every other `Core\Request` accessor.

Verification, in the order it becomes possible:

- **M2**: the `mwl check` corpus [ADR 0007](0007-explicit-type-system.md) already builds gains its own
  entries — concatenating a `tainted` value into a sink that requires the plain type is a diagnostic naming
  the qualifier and the sink; a checked `as uint`/`as` an enum's backing type on a tainted source produces
  an unqualified result with no extra syntax; `tainted string as Markup` is refused even though
  `"literal" as Markup` succeeds.
- **M7**: `Core\Request::query()`/`::post()`/`::cookie()`/`::file()` and `Core\Server::header()` return
  `tainted string`/`tainted bytes`; the path-traversal and header-injection conformance suites already on
  M7's verify list ([ADR 0017](0017-hot-reload-without-restart.md)'s neighboring milestone paragraph) gain a
  case built specifically from a live `Core\Request` value reaching a filesystem or header sink without
  laundering, and confirm it is rejected at compile time, not only caught by the runtime suite.
- **M8**: `Core\Db`'s query API rejects a tainted value at its SQL-text parameter at compile time, and
  accepts one freely at its bound-parameters argument; `Core\Html::escape`/`Markup` round-trip tested
  against the OWASP XSS filter-evasion cheat sheet strings, held to the same fuzz-corpus rigor
  [ADR 0019](0019-reflection-and-ast-parsing-are-core-features.md) already set for `Core\Ast::parse()`.
