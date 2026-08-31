# ADR 0092 — One diagnostic record, three renderings, and the sink in force picks

- **Status:** Accepted
- **Date:** 2026-08-25
- **Scope:** the one closed record model every developer-facing output is built from; the three renderings
  of it — plaintext, JSON, HTML — and which sink selects which; `Core\Log`'s level enum and write path;
  `Core\Debug::dump` and where it lands in each context; and the four transformations (redaction, control
  bytes, bidi, elision) that the model decides once for all three renderings. Not in scope: the *content*
  of what a call site records, which stays the call site's; the compile-time log field schema, which is its
  own decision and § 8 states what this ADR does to keep it possible; scoped context fields (`Log::with`),
  same; the escalation ladder's tiers, which stay [0020](0020-error-escalation-ladder.md) §§ 1–5; what the
  HTML and terminal sinks *do* once reached, which stay
  [0024](0024-taint-tracking-for-injection-sinks.md) § 5 and
  [0086](0086-core-cli-terminal-is-a-sink.md) § 1; and the per-mode defaults of `[log] format`/`level` and
  `[debug] inline`, which are [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 3's.
- **Amends:** [0064](0064-configuration-file-format.md) § 2a — the `[log]` block gains `format` and
  `level`, whose values § 2 and § 3 below fix.
  [0086](0086-core-cli-terminal-is-a-sink.md) § 1 — its substitution table becomes a property of § 1's
  record model, so all three renderings inherit one answer; § 3's colour resolution is what the
  plaintext rendering reads.
  [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md) — its predicate gains a fourth caller,
  § 1's record model; the rule itself is unchanged.
  [0020](0020-error-escalation-ladder.md) § 6 — `Core\Log::write`'s `string $level` becomes the
  `Log\Level` enum, and JSON Lines becomes the log sink's default *rendering* of the shared record rather
  than the record's only possible shape; the one-serialiser property that section exists to protect is
  strengthened rather than weakened, because the shared thing is now the record and not the bytes.
  [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) — `Core\Debug`'s `dump` was
  attributed to that ADR by the [spec's](../spec/01-core-library.md) § 16 row and is not in its scope; the
  row now points here, and 0018 keeps the coverage/trace/profile members it actually argues.
  [0028](0028-closing-the-remaining-magic-methods.md) § 4 — the guarantee is restated against
  `Core\Debug::dump` rather than PHP's `var_dump`/`print_r`, which Novis does not have; the rule is unchanged
  and the closed-mechanism refusal of a `DebugRepresentable` hook is reaffirmed by § 7.
  [0033](0033-secret-qualifier-for-confidential-values.md) — the debug-dump bullet is restated against
  `Core\Debug::dump`, and its redaction becomes a node kind in § 1's model so all three renderings inherit
  it rather than each implementing it.
  [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 3 — the sink binding table gains a
  rendering column; § 5's carrier rule is reused unchanged.
  [0079](0079-testing-is-a-language-feature.md) — a `#[Test]` run's result is a record under § 1 and gets a
  JSON rendering for CI for free.
  [docs/spec/01-core-library.md](../spec/01-core-library.md) § 16 — the `Core\Debug` row gains `dump` and
  points here; the `Core\Log`/`Core\Fatal` row's *"both are `secret` sinks"* is disambiguated against
  [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 7's *"`Core\Log` is still not a
  sink"* — they are different axes, and that row gains `write`'s signature and the `Log\Level` enum.
  [docs/implementation-plan.md](../implementation-plan.md) — M4's `var_dump`/`print_r` line item is
  renamed to this model and its plaintext rendering.
- **Amended by:** [0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 10 — § 1's
  envelope gains `count`, written by the sink rather than by the producer.

> **In short:** every language has three or four half-answers to *show me this value* — PHP has
> `var_dump`, `print_r`, `var_export` and `json_encode`, and each looks acceptable in exactly one output
> medium and unreadable in the others. Novis takes the opposite shape: **one closed record model, three
> renderings of it, and the rendering is chosen by the sink already in force rather than by the call
> site.** [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 3 already decided that every
> execution context has a sink and § 5 already decided that capturing one yields *that sink's* carrier;
> this ADR is those two rules applied one step further out, so it adds **no new carrier type and no new
> userland value type** — a rendered record is the `Cli\Text`, `Core\Html\Markup` or `mixed` that already
> existed. Five producers share the model — `Core\Log`, `Core\Debug::dump`, a `Throwable` and its trace, a
> `#[Test]` result, and a compiler diagnostic — which is what makes the four transformations that matter
> decidable **once**: [0033](0033-secret-qualifier-for-confidential-values.md)'s redaction,
> [0086](0086-core-cli-terminal-is-a-sink.md) § 1's control-byte substitution,
> [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md)'s bidi rule, and depth/length elision. A
> dump goes to the **log** by default and reaches a response body only under
> [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s development mode, which makes
> PHP's most-exploited information disclosure — a `var_dump` reaching a production response — unspellable
> rather than a matter of discipline.

## Context

- **PHP's four functions are four half-answers, and the split is by medium.** `var_dump` is readable in a
  terminal and unreadable in a browser, because it emits newline-separated text into HTML that collapses it
  to one line; `print_r` is the same trade with less type information; `var_export` exists to emit PHP
  source; `json_encode` is machine-readable and shows nothing about objects' declared shape. A developer
  picks by *where the output is going*, which is exactly the decision a runtime is better placed to make
  than a call site.
- **The medium is already known.** [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 4
  made a response body **typed** — a handler wrote `echo` (HTML), `Response::json`, or `Response::text`,
  and mixing `echo` with a typed writer is already a compile error. So the rendering is not something to
  negotiate from a `Content-Type` header at run time; it is statically known at the dump site. The same is
  true of every other context: § 3 of that ADR is a complete table of which sink is in force where.
- **`Core\Debug::dump` is one table cell with no surface behind it.** It appears once, in
  [docs/spec/01-core-library.md](../spec/01-core-library.md) § 16, attributed to
  [0018](0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) — an ADR about safepoint-shaped
  coverage and trace probes that never mentions dumping. No signature, no format, no owner.
- **`Core\Log::write` is explicitly illustrative and its level is a `string`.**
  [0020](0020-error-escalation-ladder.md) § 6 writes `Core\Log::write(string $level, …)`, which
  [0007](0007-explicit-type-system.md) and [0010](0010-enums-are-a-value-type.md) would both refuse for a
  closed set of values in any other position in the language.
- **Four transformations are currently owed by nobody in particular.**
  [0033](0033-secret-qualifier-for-confidential-values.md) says a dump redacts a `secret` property,
  [0086](0086-core-cli-terminal-is-a-sink.md) § 1 says terminal output substitutes control bytes,
  [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md) says an unterminated directional control is
  substituted at every output boundary, and nothing says what a dump's depth or length cap is. Each is
  correct; none is implemented once. Left as is, a stack trace and a log record acquire different answers
  to the same four questions within a release or two.
- **A plaintext log target is a log-forging surface and a JSON one is not.** CWE-117: a newline inside a
  value forges an entire log entry. [0020](0020-error-escalation-ladder.md) § 6 chose JSON Lines partly for
  this — its escaping is "correct on the first and only attempt" — so *adding* a human-readable rendering
  is exactly where that property could be lost, and must not be.
- **What the ecosystem got right is worth naming.** Go's `slog`, Rust's `tracing` and Serilog all converged
  on **structured at the call site, formatted at the sink**, which is the same decision as this ADR's and
  the reason it is not novel in the large. Symfony's VarDumper is the one PHP answer that genuinely works
  in a browser — collapsible, typed, class-aware — and § 3's HTML rendering is deliberately modelled on it.

## Decision

### 1. One record, and it is a closed model of content only

Every developer-facing output in Novis is a **record**: an envelope plus a tree of **nodes**. The model is
closed — a node is one of a fixed set of kinds, and there is no extension point.

The **envelope** carries what is true of the whole record and nothing about how it looks:

| Field | Notes |
|---|---|
| `ts` | RFC 3339, as [0020](0020-error-escalation-ladder.md) § 6 already fixes |
| `level` | § 2's `Log\Level` |
| `message` | plain `string`, never a qualified one ([0033](0033-secret-qualifier-for-confidential-values.md) § 4's `Throwable`-message rule, same argument) |
| `request_id` | |
| `trace_id`, `span_id` | present only when a trace is active, omitted rather than empty — [0076](0076-observability-export.md) § 6 |
| `source` | file, line, and the enclosing member |
| `count` | how many identical records this one stands for — [0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 10's coalescing counter, and the one envelope key a **sink** writes rather than a producer, which is why it is not an entry in `fields` |
| `fields` | named, each carrying a node — **not** a stringly bag; § 8 says why that matters |

A **node** is one of:

| Kind | Carries |
|---|---|
| Scalar | `null`, `bool`, `int`, `float`, `decimal`, `string`, `bytes` — the tag is the Novis type, so `"1"` and `1` are never confusable, which is the one thing `print_r` cannot do |
| Sequence, Map | an `array`'s two shapes ([0009](0009-string-and-bytes.md), [0069](0069-array-combination-is-key-type-independent.md)) |
| Object | the class name and its **declared** properties, per [0028](0028-closing-the-remaining-magic-methods.md) § 4 |
| Enum case | the enum's name and the case's, never its underlying integer ([0010](0010-enums-are-a-value-type.md)) |
| Closure | its declared signature; never a body, and never captured state ([0031](0031-callable-is-the-only-closure-type.md)) |
| Redacted | stands where a `secret`-typed value would have been — § 5 |
| Elided | what was cut and how much of it — § 5 |
| Cycle | the identity of the node it repeats, from `nvs_runtime::identity` |
| Span | a source range with a label, which is what a compiler diagnostic is made of |

**This model is content, not presentation.** It carries no colour, no indentation, no width and no
ordering-for-display; a rendering supplies all four. That separation is the whole ADR, and it is why
adding a fourth rendering later costs one implementation rather than five.

**One crate owns the model and all three renderings**, and both the runtime and the compiler front end
depend on it. It is `nvs-render`, and it exists: M4 created it with the model, the plaintext rendering and
§ 5's four transformations, `nvs-stdlib` being its first dependent through `Core\Debug::dump`.
`nvs-diagnostics`'s existing terminal renderer moves into it and becomes one of the three at M10; that
module's own doc comment already anticipates this, naming "the same layout engine to emit LSP-shaped data"
as a requirement it was built for. `nvs-runtime` depends on no `nvs-*` crate today, so the model cannot
live in `nvs-diagnostics` and the dependency runs the other way — which is also why `nvs-render`'s own one
dependency, ADR 0087's bidi predicate in `nvs-syntax`, is a **temporary** direction: that crate's module
doc owns the move that inverts it once `nvs-runtime` or `nvs-diagnostics` becomes a dependent.

### 2. `Log\Level` is five cases, with a fixed syslog mapping

```
Log\Level::Debug     Log\Level::Info     Log\Level::Warn
Log\Level::Error     Log\Level::Critical
```

An [0010](0010-enums-are-a-value-type.md) enum, replacing
[0020](0020-error-escalation-ladder.md) § 6's `string $level`:

```
Core\Log::write(Log\Level $level, string $message, array<string, mixed> $fields = []): void
```

| Case | syslog severity | Used by |
|---|---|---|
| `Debug` | 7 | `Core\Debug::dump`'s default destination (§ 4) |
| `Info` | 6 | |
| `Warn` | 4 | [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 6's public-bind record |
| `Error` | 3 | an uncaught `Throwable` |
| `Critical` | 2 | [0020](0020-error-escalation-ladder.md)'s tier-3 and tier-4 floor |

The mapping is fixed because [0020](0020-error-escalation-ladder.md) § 4 names `syslog` as a target and a
severity is not optional there.

- **`Critical` exists so the escalation ladder has a level of its own** rather than a parallel channel. A
  resource-limit fatal and a failed third-party call are not the same alerting decision, and expressing
  that difference by level is where an alerting rule can read it — a four-case roster would have pushed it
  into a field.
- **There is no `Trace` case.** Spans and sampling are [0076](0076-observability-export.md)'s, and a trace
  *level* would be a second home for that fact.
- **Eight PSR-3 cases were considered and rejected.** They are what PHP standardised on, and they would
  have made `nvs convert`'s mapping an identity — a tier-E rewrite under
  [0089](0089-convert-is-one-rule-table-with-two-modes.md) rather than tier-D. But nobody has a rule for
  choosing `notice` over `info`, or `alert` over `emergency` over `critical`; they are three ways to say
  one thing, which is the shape this project rejects everywhere else. `nvs convert` maps `notice`→`Info`
  and `alert`/`emergency`→`Critical`, in that ADR's rule table.

`[log] level` sets the minimum level written; its per-mode default is
[0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 3's.

### 3. Three renderings, and the sink in force picks

[0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 3's binding table gains a column. The
call site never names a rendering; **there is no format argument on any of § 6's five producers.**

| Sink in force | Rendering | Carrier — every one already exists |
|---|---|---|
| the terminal (the default sink everywhere) | **plaintext**, indented; coloured iff `Cli::colorDepth() != None` | `Cli\Text` ([0086](0086-core-cli-terminal-is-a-sink.md)) |
| an HTTP request writing an HTML body | **HTML** — a collapsible, typed, class-aware block | `Core\Html\Markup` ([0024](0024-taint-tracking-for-injection-sinks.md) § 5) |
| an HTTP request writing a JSON body | **JSON** | `mixed`, which `Response::json` already serialises |
| the log target — `stderr`, `file:`, `syslog` | `[log] format`: `"json"` (JSON Lines) or `"text"` | bytes |

**Zero new carrier types.** That is not an aesthetic result: it is what makes
[0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 5 apply unchanged, so
`Core\Out::capture` around a dump returns something that re-emits correctly instead of being escaped twice.

- **Plaintext colour is tty-dependent; nothing else is.** [0086](0086-core-cli-terminal-is-a-sink.md) § 3
  already resolves colour depth honouring `NO_COLOR`, `CLICOLOR_FORCE`, `FORCE_COLOR` and `TERM=dumb`, and
  the plaintext rendering reads that one answer. Structure, substitution and redaction never vary with a
  tty — § 5.
- **`[log] format` has two values, not three.** HTML is not a log *target* rendering; a web-facing log
  viewer is an HTTP response, so it reaches the HTML rendering through the response sink like everything
  else. A framework log viewer ([0082](0082-the-first-party-framework.md)) therefore gets it for free and
  needs no second implementation.
- **The HTML rendering carries its own style, inline and once per response, under the CSP nonce.**
  [0074](0074-http-defaults-safe-and-finite.md)'s default headers make a CDN-linked asset dead on arrival,
  and a debug block that fails to render because of the project's own security defaults would be an
  embarrassing kind of correct.
- **A rendering is not a fifth grammar.** [0063](0063-core-api-conventions.md) R11 fixes the library's
  grammar count at exactly four, and a grammar there is a *string the program writes that something
  parses*. A rendering is output the runtime produces. R11's count is unchanged, and so is
  [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 1's corollary that every R11 grammar
  is a sink.

### 4. `Core\Debug::dump` goes to the log, and reaches a body only in development mode

```
Core\Debug::dump(mixed ...$values): void
Core\Debug::render(mixed $value): <the carrier of the sink in force>
```

| Context | `[debug] inline` | Where a dump lands |
|---|---|---|
| a CLI program, a scheduled script, a job worker, a `#[Test]` method | — | **stderr**, plaintext |
| an HTTP request, HTML body | `false` (production default) | one record at `Log\Level::Debug` |
| an HTTP request, HTML body | `true` (development default) | a collapsible block appended to the body, **and** the log record |
| an HTTP request, **JSON body** | either | the log record only — **never inline** |

- **CLI dumps go to stderr, not stdout**, so `prog | jq` and `prog > out.txt` keep working. `var_dump`
  writing to stdout is a small thing that makes PHP CLI tools unpipeable while debugging, and there is no
  reason to inherit it.
- **A JSON body is never modified, in either mode.** [0085](0085-openapi-is-generated-from-the-route-table.md)
  generates the OpenAPI document from the route table and makes an `#[Api]` that contradicts the code a
  compile error; injecting a `debug` key would make the *served* shape disagree with the *published*
  contract in exactly the environment where clients are being written against it, and a strict client
  validator would then pass in production and fail in development — the worst direction for a bug to point.
  The consequence is worth stating plainly: **a development-mode API response is byte-identical in shape to
  its production counterpart.**
- **`[debug] inline` is `RuntimeTighten`.** A request may turn its own inline output *off* and can never
  turn it on, so the mode's default is the only thing that can enable it — and
  [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 5's ceiling is what bounds
  reaching that mode at all.
- **`render` exists so a dump can be embedded rather than written**, returning the sink's carrier by
  [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 5's rule verbatim. It is one member,
  not a second mechanism.

**This is the security half of the ADR.** PHP's most-exploited information disclosure is not a bug in
`var_dump`; it is that `var_dump` writes to output, so a forgotten call and a production deployment are
enough. Here the forgotten call writes a log line, and the spelling that would put it in a response does
not exist outside a mode whose ceiling is closed by default.

### 5. Redaction, control bytes, bidi and elision are decided once, in the model

All four are properties of the **record**, applied when it is built, before any rendering sees it. Every
rendering therefore inherits identical answers, and none may weaken one.

| Transformation | Rule | Owner |
|---|---|---|
| **Redaction** | a property whose *declared* type carries `secret` becomes a Redacted node; a `secret` value passed at a call site is refused by `nvs check` | [0033](0033-secret-qualifier-for-confidential-values.md) § 4 |
| **Control bytes** | C0 except `LF`/`TAB` → its U+2400 Control Picture, `DEL` → `␡`, a C1 code point → `�` | [0086](0086-core-cli-terminal-is-a-sink.md) § 1's table, unchanged |
| **Bidi** | an unterminated directional control → `�`; a balanced one passes through | [0087](0087-unbalanced-bidi-is-rejected-at-every-boundary.md), via `nvs_render::bidi`'s one predicate |
| **Elision** | depth and per-node length caps, replacing what is cut with an Elided node naming how much | this ADR |

- **Control-byte substitution applies to the JSON rendering too**, where framing already makes it
  unnecessary for safety. Uniformity is the point: a value must not read differently depending on which
  rendering someone is looking at, or the renderings stop being views of one record. In JSON a directional
  control additionally escapes as `‮`, so a log viewer that does not re-run the predicate is still
  safe.
- **This is what closes CWE-117 for the plaintext rendering.** A human-readable log line is *not* `"$k=$v"`
  concatenation — it renders nodes whose control bytes are already substituted, so a newline inside a
  tainted value cannot forge an entry. The property [0020](0020-error-escalation-ladder.md) § 6 chose JSON
  Lines to guarantee is preserved when a human-readable target is added, which is the condition on adding
  one at all.
- **Elision belongs to the model precisely so the renderings agree on what was cut.** PHP and Python
  truncate per formatter, so the same value is complete in one output and truncated in another and no
  reader can tell which. Here a cut is a node, so it renders as a cut in all three.
- **A cycle is an identity, not a `*RECURSION*` string.** `nvs_runtime::identity` already exists
  ([0090](0090-one-equality-operator-and-disjoint-types-do-not-compile.md) § 3 lowers object equality to
  it), so the HTML rendering can collapse or link the repeat and the JSON rendering can emit a reference —
  neither of which PHP's marker permits.

**Qualifiers cost this ADR nothing, and that is a result rather than an accident.**
[0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 1's predicate already classifies a log
field and a dumped value as *data* — the record frames them — so `tainted` flows in freely and **the
rendering is what makes it safe**. That is the argument for the renderings being non-bypassable: there is
no `dumpRaw`, and no rendering may be selected by an argument.

### 6. Five producers, one model

| Producer | What it builds | Milestone |
|---|---|---|
| `Core\Log::write` | a record with the caller's fields | M8 |
| `Core\Debug::dump` | a record at `Debug`, one node per argument | M4 |
| a `Throwable` and its trace | a record at `Error`, frames as Sequence-of-Object nodes | M4 |
| a `#[Test]` result ([0079](0079-testing-is-a-language-feature.md)) | a record per assertion, the expected and actual as sibling nodes | M4S |
| a compiler diagnostic (`nvs check`) | Span nodes over a source map | M10 |

Two of these buy something concrete beyond consistency, and they are the reason the scope is five and not
two:

- **A `#[Test]` failure renders as a coloured diff locally, as JSON in CI, and as HTML in a web runner**,
  with no reporter written for any of them.
- **`nvs check` gains a JSON rendering that `nvs-lsp` consumes** ([0016](0016-ide-integration.md),
  [0040](0040-vscode-deep-tooling-and-resilient-parsing.md)), which
  [`nvs-diagnostics`](../../crates/nvs-diagnostics/src/render.rs)'s own module doc already names as a
  requirement it was designed toward.

**The `Throwable` case is the one that would have leaked if the scope had been narrower.** It is the
most-read diagnostic output in any language, and leaving it outside would have meant a second
implementation of § 5's redaction and a second of its substitution — the duplication this ADR exists to
prevent.

### 7. What this deliberately does not add

- **No customization hook.** No `DebugRepresentable`, no `__debugInfo`, no per-class renderer.
  [0028](0028-closing-the-remaining-magic-methods.md) § 4 closed this with its reasoning, and § 1's model
  being closed is that decision expressed as a data type. A dump shows a class's real declared properties
  and their real current values, with § 5's four transformations and nothing else.
- **A dump does not call `Stringable`.** [0028](0028-closing-the-remaining-magic-methods.md) settled that a
  dump shows real state; a `toString` result would be a second, prettier, possibly-lying view of it.
- **No format argument, anywhere.** Not on `dump`, not on `write`, not on `render`. The sink in force is
  the only input, which is what keeps the five producers from each growing a `$format` parameter and the
  four renderings from becoming twenty.
- **No fourth rendering.** Adding one is a change to one crate and this ADR, not to any call site — which
  is the property that makes refusing one now cheap to revisit later.
- **A log write is charged, and a shed write is counted.** A hot error path filling a disk is a real
  outage. A record is charged to the request's budget ([0006](0006-isolated-script-execution.md)) and burst
  shedding is [0075](0075-core-ratelimit.md)'s existing mechanism; what this ADR fixes is the direction —
  **a shed record increments a counter [0076](0076-observability-export.md) exports**, because a silently
  dropped log line is worse than a counted one. The numbers are M8's, with its own guard test.

### 8. What is kept possible, deliberately

Two decisions were separated out and are named here so that this ADR does not foreclose them:

- **A compile-time log field schema.** `fields` is a **named, typed** structure in § 1's model, never
  `array<string, mixed>` at the core — the open type on `Core\Log::write`'s parameter is a call-site
  convenience over it, exactly as [0033](0033-secret-qualifier-for-confidential-values.md) § 4 already
  treats that parameter for its own check. Because the names and types exist in the model, a later decision
  can have `nvs check` collect every call site — literal arguments are already folded by
  [0057](0057-intrinsic-literal-folding.md), and reading declared types off a shape is already
  [0071](0071-derived-codecs.md)'s machinery — build the program's whole log schema at compile time, and
  refuse two call sites that use one field name with two types. No mainstream logger can do this. It is a
  content decision rather than a representation one, which is why it is not taken here.
- **Scoped context fields** (`Log::with({tenant: …}, fn() => …)`), where a record inherits the enclosing
  scope's fields. [0072](0072-core-task-structured-concurrency.md) already defines exactly where such a
  scope would end and [0076](0076-observability-export.md) already injects `trace_id`/`span_id` by the same
  shape, so the envelope in § 1 accommodates it without change.

## Consequences

- **Four PHP functions become one member, and it reads correctly in every medium.** `var_dump`, `print_r`,
  `var_export` and `json_encode`-as-a-debug-tool collapse into `Core\Debug::dump`. That is a reduction in
  language surface, not an addition — priority 4 improves.
- **The four transformations have one implementation and one test surface.** The conformance obligation is
  a table of every node kind against every rendering, which is finite because the model is closed. That
  table is the whole defence against the three renderings drifting apart, and it is why closing the model
  mattered more than any individual node kind in it.
- **The cost on the request path, as [0004](0004-memory-for-simplicity.md) requires it be stated.** A
  record is built before it is rendered, so a log write allocates a bounded tree rather than formatting
  straight to bytes — one extra allocation and one extra traversal per record, bounded by § 5's caps and
  charged to the request. The JSON Lines path renders streaming from the tree without materialising an
  intermediate string, so the hot path pays the tree and not a second copy. This is a priority-3/5 spend
  buying priority-1 (a dump that cannot reach a production response, a plaintext target that cannot be
  forged) and priority-4 (one member instead of four), which is the direction
  [0004](0004-memory-for-simplicity.md) prescribes.
- **A `Core\Log::write` call site changes shape.** `"error"` becomes `Log\Level::Error`. Every existing
  example in [0044](0044-core-process-argv-only-no-shell.md) and [0075](0075-core-ratelimit.md) is
  rewritten with this ADR, and `nvs convert` maps PSR-3's eight names onto § 2's five.
- **`nvs-diagnostics`'s terminal renderer is refactored rather than rewritten.** It becomes the plaintext
  rendering of the shared model; its span-and-label layout survives as § 1's Span node. That is real work
  at M10 and it is the only existing code this ADR disturbs.
- **A new crate exists** — one more than the workspace has today, created at its milestone. Justified by
  the dependency direction in § 1: `nvs-runtime` depends on no `nvs-*` crate, so a shared model cannot live
  in an existing one.

## Alternatives rejected

- **A format argument on each producer** — `dump($x, "html")`. The obvious design, and the one every PHP
  framework arrives at. Rejected because the call site is the one place that does *not* know where the
  output is going: the same handler runs under `nvs serve`, under a `#[Test]`, and inside a `spawn script`
  isolate, and [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 3 already answers the
  question correctly in all three.
- **Content negotiation from the `Content-Type` header at dump time.** Rejected as strictly weaker than
  what already exists: [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 4 makes the body
  typed, so the rendering is statically known, and sniffing a header would reintroduce a run-time decision
  where there is a compile-time fact.
- **Logs and dumps only, leaving `Throwable`, tests and diagnostics outside.** The smallest possible ADR.
  Rejected in *Decision* § 6: it duplicates redaction and substitution rather than removing a duplication,
  which is the opposite of the goal.
- **Eight PSR-3 levels.** § 2 holds the reasoning and what it costs `nvs convert`.
- **A `debug` key injected into a JSON response body in development mode.** § 4 holds the reasoning; the
  OpenAPI contradiction under [0085](0085-openapi-is-generated-from-the-route-table.md) is what decided it.
- **A response header carrying a dump id** (`X-Nvs-Debug`), the shape Symfony's `X-Debug-Token` uses.
  Keeps the body intact *and* the dump discoverable. Not taken: it is a second mechanism — an id, a lookup,
  and a viewer to look it up in — and the viewer belongs to [0082](0082-the-first-party-framework.md),
  which has not started. Cheap to revisit once it has.
- **`DebugRepresentable`, or a per-class dump hook.** Rejected once already by
  [0028](0028-closing-the-remaining-magic-methods.md) § 4; § 7 reaffirms it rather than reopening it.
- **Rendering `Stringable` in a dump.** Same section, same reason.

## Verification

- **M4:** the record model and the plaintext rendering exist; `Core\Debug::dump` writes to **stderr** in a
  CLI program, and a fixture asserts stdout is byte-empty while stderr holds the dump.
- **M4:** a dumped object shows every declared property; a property whose declared type carries `secret`
  renders as the redaction placeholder in all three renderings, and a `secret` value at a `dump` call site
  is refused by `nvs check` ([0033](0033-secret-qualifier-for-confidential-values.md)).
- **M4:** a dumped `string` containing `ESC`, a bare `CR`, a C1 code point and an unterminated `U+202E`
  renders with [0086](0086-core-cli-terminal-is-a-sink.md) § 1's substitutions in **all three** renderings
  — the JSON one included, where the bidi control additionally appears as `‮`.
- **M4:** a value exceeding the depth or length cap renders an Elided node naming what was cut, and the
  plaintext, JSON and HTML renderings agree on the cut. A cyclic graph renders a Cycle node, not an
  infinite traversal.
- **M4:** an uncaught `Throwable` renders through the same model; a `secret`-typed property on an object in
  a stack frame is redacted there too.
- **M6:** `[log] format` and `[log] level` parse, and resolve to
  [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 3's per-mode defaults.
- **M7:** with `[debug] inline = true`, a dump in a request writing an HTML body appends a collapsible
  block carrying its own inline style under the CSP nonce; with `inline = false` the body is byte-identical
  to the same request with no `dump` call at all, and the log holds one `Debug` record.
- **M7:** a dump in a request writing a `Response::json` body **never** alters the body, under either value
  of `[debug] inline` — the assertion is byte-equality of the response against the same handler with the
  `dump` removed.
- **M7:** `echo Core\Out::capture(fn() => Core\Debug::dump($x))` in a request produces singly-escaped
  output, per [0088](0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 5.
- **M8:** `Core\Log::write` at each of § 2's five levels writes JSON Lines under `format = "json"` and the
  plaintext rendering under `format = "text"`; a field value containing `LF` cannot produce a second parsed
  entry in the plaintext target (the CWE-117 case).
- **M8:** the [0020](0020-error-escalation-ladder.md) tier-4 floor and an ordinary `Core\Log::write` at
  `Critical` produce records a single Loki query matches — the one-serialiser property that ADR's § 6
  exists to protect, restated against the record.
- **M8:** a shed record increments the counter [0076](0076-observability-export.md) exports; the count is
  never zero when records were dropped.
- **M10:** `nvs check` renders the same diagnostic as an annotated snippet on a terminal and as the JSON
  `nvs-lsp` consumes, from one record.
