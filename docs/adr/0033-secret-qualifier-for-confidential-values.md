# ADR 0033 — `secret`: a second compile-time qualifier for confidential values, composable with `tainted`

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** a `secret` compile-time qualifier on `string`/`bytes`, independent of and composable with
  [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s `tainted`; how it enters, propagates, and is
  removed; the sinks that refuse a `secret` value (HTML/response output, `Core\Log`, debug-dump output,
  `Throwable` messages, `serialize()`/the isolate-crossing boundary); the redaction `var_dump()`/`print_r()`
  owe a `secret`-qualified property.
- **Amends:** [0007](0007-explicit-type-system.md) § 2 — the conversion table's checked-conversion row now
  strips `secret` on success, the same total/checked shape [0024](0024-taint-tracking-for-injection-sinks.md)
  already gave `tainted`; every other row is unchanged.
  [0024](0024-taint-tracking-for-injection-sinks.md) § 1 — the `qualified_type` grammar production gains a
  second, independent optional qualifier: `qualified_type := 'secret'? 'tainted'? scalar_type | <every other
  atom in ADR 0007 § 3, unqualified>`. § 2's checked-conversion laundering rule now also strips `secret`.
  § 5 — a `secret`-qualified value reaching a `Markup`-building interpolation position (the inline
  `<?= expr ?>` slot or a templating helper) is refused with a diagnostic, not auto-escaped-and-displayed;
  auto-escape neutralizes structure, not exposure, so it is the wrong tool for this qualifier and § 5's "never
  distinguishes tainted from untainted" sentence gets one carve-out for `secret`.
  [0020](0020-error-escalation-ladder.md) § 6 — `Core\Log::write`'s `fields: array<string, mixed>` parameter
  stays open per [0007](0007-explicit-type-system.md) § 6, but `mwl check` now inspects the literal
  expressions passed at that call site (not just the declared parameter type) for a statically-`secret`
  operand and refuses it — see § 4 below for why this sink needs call-site inspection instead of a
  parameter-type refusal.
  [0028](0028-closing-the-remaining-magic-methods.md) § 4 — `var_dump()`/`print_r()` no longer
  unconditionally show "a class's real declared properties and their real current values": a property whose
  declared type carries `secret` is shown as a fixed redaction placeholder instead of its value. This is a
  built-in, type-driven rule keyed on the declared type, not a per-class hook — it does not reopen § 4's
  rejection of a `DebugRepresentable`-style customization interface, the same distinction
  [0019](0019-reflection-and-ast-parsing-are-core-features.md) already drew for `Core\Reflect`.
  [docs/implementation-plan.md](../implementation-plan.md) M1 — gains a second qualifier-grammar addition
  alongside `tainted`'s, landing after M1's own fuzz/corpus verification was already reported done, the same
  situation [0024](0024-taint-tracking-for-injection-sinks.md)'s *Consequences* already flagged once. M4 —
  the `var_dump`/`print_r` line item gains this ADR's redaction rule. M8 — the `Core\Log` line item gains
  this ADR's call-site inspection rule.
- **Relates to:** [0004](0004-memory-for-simplicity.md) (compile-time-only, erased before codegen — the same
  free-security argument [0024](0024-taint-tracking-for-injection-sinks.md) already made, applied to a
  second axis), [0009](0009-string-and-bytes.md) § 4 (the scalar payload pulled out of `mixed` may now also
  be `secret string`/`secret bytes`, alongside `tainted`'s own addition there), [0012](0012-no-superglobals.md)
  (contrast: every `Core` accessor there returns `tainted` ambiently; nothing returns `secret` ambiently — see
  § 1), [0015](0015-no-name-aliasing.md) (precedent for "exactly one canonical spelling," which is why
  combined qualifiers accept only one keyword order, not two), [0020](0020-error-escalation-ladder.md) § 6
  (the JSON-Lines writer this ADR's `Core\Log` sink rule constrains), [0022](0022-definite-property-initialization.md)
  (precedent for a compiler-enforced guarantee about what a declared property can hold), [0023](0023-clone-serialize-and-cross-boundary-copy.md)
  (`serialize()`/`unserialize()` share one graph-copy operation with the `spawn`/`spawn worker`/
  `spawn script` boundary — this ADR's § 4 refuses `secret` at that one operation, both callers, rather than
  drawing a new line between them), [0028](0028-closing-the-remaining-magic-methods.md) (the `__debugInfo`
  guarantee this ADR amends, and the precedent that a closed, type-driven rule is not the customization
  surface that section already rejected)

> **In short:** `secret string`/`secret bytes` join `tainted string`/`tainted bytes` as a second, independent
> compile-time qualifier — `secret` and `tainted` answer different questions (*can I trust where this came
> from* vs. *where is this allowed to go*) and a value can carry either, both, or neither
> (`secret tainted string` is valid). Like `tainted`, `secret` is erased before codegen: no runtime
> representation, no per-op cost, checked once by `mwl check`. Unlike `tainted`, a checked `as` conversion
> strips `secret` on success for the same reason it strips `tainted` — consistency with the existing rule was
> chosen over a stricter one for this first cut; see *Alternatives rejected* for the case against it and
> *Revisiting* for when to reconsider. `secret` has no ambient source the way `tainted` has
> [ADR 0012](0012-no-superglobals.md)'s five accessor classes: nothing in MWL is host-populated
> ([0012](0012-no-superglobals.md)), so a value becomes `secret` only where a developer spells it on a
> declaration — a config-loading helper that reads a credential is expected to declare its own return type as
> `secret string`. Four sinks refuse a `secret` value by default: HTML/response output (refused outright, not
> auto-escaped — escaping doesn't restore confidentiality), `Core\Log` (the opposite of `tainted`'s "logging
> it is the point" stance), debug-dump output and `Throwable` messages (a redaction placeholder, not the real
> value), and `serialize()`/the isolate-crossing boundary (one refusal for the one operation
> [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already unified). The only way to remove
> `secret` outside a checked conversion is a narrow, named `Core` function —
> `Core\Secret::reveal(secret string, string $reason): string` (and a `bytes` overload), modeled directly on
> `Core\Taint::assertTrusted` — or a purpose-built helper that consumes a secret and returns a genuinely
> non-secret derivative, such as a password-hashing function.

## Context

- `tainted` ([ADR 0024](0024-taint-tracking-for-injection-sinks.md)) answers "is this value's shape safe for
  a sink," not "is this value confidential" — an API key can be perfectly well-shaped and still catastrophic
  to leak, while a username can be safe to display yet still attacker-controlled. Neither qualifier covers
  the other's case; a submitted password is both at once.
- `secret` cannot reuse `tainted`'s laundering rule on principle — a checked conversion proves shape safety,
  not reduced confidentiality, so an API key that parses `as uint` is exactly as sensitive afterward. This
  ADR keeps the same removal-on-conversion rule anyway, for grammar/implementation consistency rather than
  because the reasoning transfers (see *Alternatives rejected*, *Revisiting*).
- `secret` also cannot reuse `tainted`'s *source* story: [ADR 0012](0012-no-superglobals.md)'s five accessor
  classes enumerate every place untrusted data ambiently enters, which is what lets `tainted` attach
  automatically. There is no equivalent enumeration for secrecy, so `secret` has no ambient source at all —
  which `Core` accessors should return it by convention is deferred stdlib design (*Revisiting*).
- `Core\Log`'s open `fields: array<string, mixed>` bag ([ADR 0020](0020-error-escalation-ladder.md) § 6) lets
  `tainted` values through freely, since logging attacker input is the log's purpose. `secret` needs the
  opposite default, which a parameter-type refusal can't express on an already-`mixed` parameter — forcing
  `mwl check` to inspect `Core\Log::write`'s call-site argument expressions instead, a mechanism `tainted`
  never needed.
- [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already unified `serialize()`/`unserialize()`
  and the `spawn`/`spawn worker`/`spawn script` boundary into one operation to avoid two implementations to
  keep correct. This ADR refuses `secret` at that one operation for both callers rather than reopening the
  seam (*Alternatives rejected*, *Revisiting*).

## Decision

### 1. `secret` is a second, independent compile-time qualifier — no ambient source

```
scalar_type    := 'string' | 'bytes'
qualified_type := 'secret'? 'tainted'? scalar_type | <every other atom in ADR 0007 § 3, unqualified>
```

`secret` and `tainted` are independent bits, not a combined enum: a value can be `string` (neither),
`tainted string`, `secret string`, or `secret tainted string`. When both are spelled together, **`secret`
comes first** — this is the only accepted order, the same "exactly one canonical spelling" stance
[ADR 0015](0015-no-name-aliasing.md) already takes for names; `tainted secret string` is a diagnostic naming
the required order, not a second valid spelling of the same type. Like `tainted`, this needs a reserved
keyword in the lexer and a new grammar production in `mwl-syntax`'s type grammar — landing after M1's own
fuzz/corpus verification was already reported done, the same situation
[ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s *Consequences* flagged for `tainted` itself, so this
is a second instance of an already-accepted cost, not a new kind of one.

Unlike `tainted`, **nothing in MWL grants `secret` ambiently.** [ADR 0012](0012-no-superglobals.md)'s five
accessor classes are the reason `tainted` can attach itself automatically — every one of them is a named,
enumerable place untrusted data enters. There is no equivalent list for secrecy: `Core\Env::get()`,
`Core\Session`, a future `Core\Db` row, and a source-literal string all look identical to the type checker
whether or not their content happens to be a credential. `secret` therefore only ever appears where a
developer spells it on a declaration — a parameter, return type, property, or local — the same way `uint`
or `readonly` do. A helper that loads an API key from configuration is expected to declare its own return
type `secret string`; the language gives no free ride the way it does for request data.

### 2. Propagation: `secret` poisons; a checked conversion launders it, same as `tainted`

Any operation combining a `secret` operand with a non-`secret` one — concatenation, interpolation, a string
function — produces a `secret` result, the identical poisoning shape [ADR 0024](0024-taint-tracking-for-injection-sinks.md)
§ 2 already defines for `tainted`, applied to this axis independently (a `secret tainted string` interpolated
with a plain `string` stays `secret tainted string`; poisoning tracks each axis on its own).

A successful checked `as` conversion (`as uint`, `as int`, `as float`, `as bool`, an enum's backing type)
**removes `secret` on success**, exactly mirroring § 2's `tainted` rule, for grammar and implementation
consistency rather than because the underlying justification transfers — it does not (see *Context* and
*Alternatives rejected*). `bytes as string` / `string as bytes` preserve `secret` across either direction,
same as they preserve `tainted`.

### 3. Removal outside a checked conversion is a narrow, named `Core` function — never generic

`Core\Secret::reveal(secret string, string $reason): string` (and a `bytes` overload) is the one narrow
escape hatch, modeled directly on `Core\Taint::assertTrusted` — forbidden by default, rare, greppable, and
carrying a written reason at the call site. There is deliberately no generic `unwrap()`/`expose()`: the same
"a catch-all invites false confidence" reasoning [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 3
already gives for laundering functions.

A second, more common removal path is a **purpose-built function that consumes a `secret` value and returns
a genuinely non-secret derivative** — a password-hashing function is the canonical example: it takes
`secret string`, and its output (a hash suitable for storage) is not itself confidential in the same way, so
it may declare a plain `string` return. This is not a loophole; it's the ordinary shape of "the secret goes
in, something safe to keep comes out," and each such function's author is responsible for that being true,
the same trust `Core\Html::escape()`'s author already carries for `tainted`.

### 4. Sinks that refuse a `secret` value by default

- **HTML/response output** — refused outright, with **no auto-escape bypass**. This is the one place
  `secret`'s enforcement actively diverges from `tainted`'s: [ADR 0024](0024-taint-tracking-for-injection-sinks.md)
  § 5 auto-escapes any non-`Markup` value on the way into a `Markup`-building position, because escaping
  fully neutralizes the risk taint tracking cares about (injection). It does nothing for confidentiality — an
  escaped credential is still a leaked credential, just HTML-safe. A `secret`-qualified value reaching a
  `Markup`-building interpolation position is therefore a compile-time diagnostic, full stop. (A `secret`
  variable can never separately reach `Markup` via `as Markup` either — that conversion already accepts only
  a source-literal token, per § 5's existing rule, which a `secret`-qualified binding never is.)
- **`Core\Log`** — the opposite default from `tainted`, which ADR 0024 § 4 explicitly wants logged. A
  `secret`-qualified value passed at a `Core\Log::write()` call site — including inside a `fields` array
  literal, whose declared parameter type stays `array<string, mixed>` by design — is refused by `mwl check`
  inspecting that call site's argument expressions, not by the parameter's declared type (see *Context* for
  why the two mechanisms differ). Only `Core\Secret::reveal()`'s output may legitimately reach a log field
  once a developer has explicitly said so.
- **Debug-dump output** (`var_dump()`/`print_r()`-equivalent) — a property whose *declared* type carries
  `secret` is shown as a fixed redaction placeholder (illustrative: `secret(redacted)`) instead of its
  current value, amending [ADR 0028](0028-closing-the-remaining-magic-methods.md) § 4's "always show real
  declared properties and their real current values" guarantee for this one qualifier. This is a built-in,
  type-keyed rule the dump implementation applies uniformly — not a per-class hook, and not a reopening of
  § 4's rejected `DebugRepresentable`-style customization interface, the same "closed mechanism, not an
  overridable one" distinction every prior magic-method closure in this project already draws.
- **`Throwable` messages** — a `Throwable`'s message parameter (constructor argument, and anywhere a message
  is later composed) requires the plain, unqualified type, the same "sink requires plain type" shape
  [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 4 already uses for `Core\Db`'s query text and
  `Core\Http`'s header setter. A `secret` value can't be concatenated or interpolated into an exception
  message without `Core\Secret::reveal()` first — closing the common real-world leak of a credential ending
  up in a stack trace or an error page.
- **`serialize()` and the `spawn`/`spawn worker`/`spawn script` boundary** — refused at the one recursive
  graph-copy operation [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) already defines, for both
  its callers alike, rather than drawing a new distinction between "crossing to a live isolate" and
  "externalizing to bytes." `Core\Secret::reveal()` before the call is the intended escape when a worker
  genuinely needs a credential to do its job — explicit, greppable, and it puts the decision at the one call
  site where it belongs. See *Alternatives rejected* and *Revisiting* if this proves too restrictive for
  legitimate worker-credential patterns in practice.

**`Core\Db`'s bound-parameters argument, `Core\Process`'s argv, and `Core\Http`'s outgoing request
headers/body are deliberately not in this list.** A credential legitimately needs to reach a database driver,
a subprocess, or an outbound HTTP call — refusing `secret` there by default would make the qualifier
unusable for its own primary purpose. This ADR closes *accidental* exposure (display, logs, dumps, error
messages, cross-boundary copies a developer didn't think about), not *intentional, narrow* use of a secret
for the job it exists to do. A future ADR or stdlib design may narrow this further per *Revisiting* if a
specific one of these proves to be a real leak vector in practice.

## Consequences

**Positive**

- Two orthogonal security questions — trust and confidentiality — each get their own zero-cost, checked type
  axis instead of being conflated into one, or left to code review.
- The most common real-world credential leaks (an error page embedding a DB password, a log line capturing
  an API key, a debug dump showing a session secret) become compile errors at the exact call sites developers
  already write, mirroring [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s own headline benefit for
  injection.
- `secret` and `tainted` compose for the case that actually matters most in practice — a submitted password —
  without needing a third combined concept.

**Negative**

- **A second grammar addition to `mwl-syntax` after M1 was already reported feature-complete, and a second
  time the milestone's own verification needs revisiting before it can be called done against this ADR's
  scope** — the same cost [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s *Consequences* already
  paid once for `tainted`, paid again here.
- **The `Core\Log` sink needs call-site argument inspection, not a parameter-type refusal** — a checker
  mechanism strictly more complex than anything `tainted` required, because `tainted` was never designed to
  need refusing inside an already-`mixed` parameter. This is new surface for `mwl check`, not a reuse of
  `tainted`'s existing machinery.
- **Checked `as` conversions strip `secret` even though the underlying justification (shape-proof implies
  safety) does not transfer from `tainted`.** A `secret string` PIN or numeric credential that round-trips
  through `as uint` becomes a plain, loggable, displayable `uint` with no diagnostic at all. This is a known,
  accepted gap, chosen for grammar/implementation consistency with `tainted` rather than discovered late —
  see *Alternatives rejected* and *Revisiting*.
- **No ambient source list** means `secret` protects only what a developer remembers to annotate. Unlike
  `tainted`, there is no [ADR 0012](0012-no-superglobals.md)-style enumeration of "every place a secret can
  enter" to lean on — this is a real usability gap relative to `tainted`'s coverage, not a design oversight;
  closing it further is stdlib work (see *Revisiting*).
- **`serialize()`/isolate-crossing refuses `secret` unconditionally**, which may add real friction to a
  legitimate pattern (a `spawn worker` that exists specifically to isolate credential handling) in exchange
  for keeping [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md)'s one-operation unification intact.

## Alternatives rejected

- **Keep `secret` through checked `as` conversions, unlike `tainted`** — the technically correct answer to
  the shape-vs-confidentiality distinction in *Context*. Rejected for this first cut in favor of the simpler,
  `tainted`-consistent rule; flagged in *Revisiting* rather than silently dropped.
- **Split `serialize()`-to-bytes from the live `spawn`/`spawn worker` boundary**, allowing `secret` to cross
  the latter but not the former. Rejected to preserve [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md)'s
  "one operation, two callers" unification. Flagged in *Revisiting*.
- **An opaque `Core\Secret` wrapper value type instead of a qualifier.** Would enable runtime memory zeroing
  later, at the cost of a second mechanism alongside `tainted`'s qualifier style plus a wrapper's usual
  friction (no implicit interpolation, explicit unwrap everywhere). Rejected as heavier than the problem
  needs.
- **Runtime memory zeroing of `secret` values on scope exit.** Rejected outright: needs a destructor-shaped
  hook, and [ADR 0028](0028-closing-the-remaining-magic-methods.md) § 2 already rules out destructors of any
  kind for the same reasons (no sound throw-reporting spot, undoes the wholesale-heap-drop request model).
  `secret` stays compile-time-only and erased before codegen, like `tainted`, per
  [ADR 0004](0004-memory-for-simplicity.md)'s ordering.

## Revisiting

- **Whether checked `as` conversions should stop stripping `secret`.** If real MWL programs show numeric or
  otherwise-convertible secrets (PINs, tokens that happen to parse as an integer) leaking past this rule in
  practice, reconsider the stricter alternative rejected above — likely by having the conversion preserve
  `secret` while still stripping `tainted`, since the two axes have no reason to share a removal rule once
  the inconsistency cost is judged worth paying.
- **Whether `serialize()`/the isolate boundary should split** into "refuse `secret` only when externalizing
  to bytes, allow it across a live `spawn worker`/`spawn script` arena." Revisit if the blanket refusal proves
  to be real friction for a credential-isolating worker pattern, weighed against reopening
  [ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md)'s unification.
- **Which built-in `Core` accessors, if any, should return `secret` by convention** — a future
  `Core\Env`/config class distinguishing `get()` from a `secret()`-returning accessor, or a `Core\Db` column
  type hint — is real stdlib design, deferred to whichever milestone designs that class's real API, the same
  deferral [ADR 0012](0012-no-superglobals.md) and [ADR 0024](0024-taint-tracking-for-injection-sinks.md)
  already used for their own accessor rosters.
- **The exact `Core\Secret` function roster** (`reveal`, a password-hashing helper, whatever else stdlib
  design turns up) is illustrative only here, due at whichever milestone builds `Core`'s credential-handling
  surface — the same status [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s laundering roster
  already carries.

Verification, in the order it becomes possible:

- **M1**: `mwl ast` parses `secret string`/`secret bytes` and `secret tainted string`/`secret tainted bytes`
  in every declaration slot [ADR 0007](0007-explicit-type-system.md) already requires a spelled type for; the
  qualifier round-trips through an AST snapshot test the same way `tainted` already does; `tainted secret
  string` (wrong order) produces a diagnostic naming the required `secret`-before-`tainted` order.
- **M2**: the `mwl check` corpus gains cases proving `secret` poisons through concatenation/interpolation
  independently of `tainted`; a checked `as uint`/`as` an enum's backing type strips `secret` (and `tainted`,
  if present) with no diagnostic; a `secret`-qualified value reaching a `Markup`-building interpolation
  position is refused even though the equivalent `tainted`-only value is auto-escaped; a `secret` value passed
  as a `Throwable` message argument is refused.
- **M4**: `var_dump()`/`print_r()` show a fixed redaction placeholder for a `secret`-qualified property
  instead of its value, while every other declared property (including `tainted`-only ones) still shows its
  real value per [ADR 0028](0028-closing-the-remaining-magic-methods.md)'s unchanged general rule.
- **M5** (the plan's own milestone for `spawn`/cross-core worker dispatch, `spawn script`, and
  `serialize()`/`unserialize()` sharing the graph-copy walk): a `secret`-qualified value passed across the
  `spawn worker`/`spawn script` boundary, or into `serialize()` directly, is refused at compile time with a
  diagnostic naming this ADR; the identical value wrapped through `Core\Secret::reveal()` first crosses
  successfully.
- **M8**: a `secret`-qualified value passed into a `Core\Log::write()` `fields` array literal is refused by
  `mwl check` at that call site despite the parameter's declared `array<string, mixed>` type; a
  `Core\Secret::reveal()`-derived value is accepted there and everywhere else this ADR's sinks refuse the
  qualified form.
