# ADR 0055 — Extension manifests carry `tainted` and `secret` qualifiers; an extension can only tighten

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** how [ADR 0024](0024-taint-tracking-for-injection-sinks.md)'s and
  [ADR 0033](0033-secret-qualifier-for-confidential-values.md)'s compile-time qualifiers behave at a Tier 1
  extension call site, and what the `mwl.manifest`/WIT world must be able to express. Not in scope: the
  rest of the WIT world's shape, which M8 authors.
- **Amends:** [0003](0003-extension-system.md) — the manifest registers classes whose static methods and
  constants enter the compiler's symbol table; those signatures now carry a qualifier axis.
  [0024](0024-taint-tracking-for-injection-sinks.md) — § 3's "only a `Core` function may launder" is
  confirmed and given teeth at the extension boundary; § 4's sink roster gains a declarable extension form.
  [0033](0033-secret-qualifier-for-confidential-values.md) — its refusal list gains the extension boundary,
  alongside serialize and isolate-crossing.
- **Amended by:** 0081 — its only-ever-tighten rule now governs every package, not only an extension; the
  fold is applied below.
- **Relates to:** 0006, 0023, 0051

> **In short:** [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 2 already says any operation
> combining a tainted operand with an untainted one produces a tainted result, and an extension call is
> such an operation — so **contagion applies at the boundary automatically, with nothing declared**. The
> manifest declares only two *deviations*: a parameter that **refuses** `tainted` (the extension is a
> sink), and a return that is **always** tainted regardless of its arguments (the extension is a source).
> There is deliberately **no manifest form that removes a qualifier** — an extension can never launder,
> because ADR 0024 § 3 reserves that for a `Core` function whose contract names one sink, and a third-party
> component asserting "this is safe for HTML" is exactly the false confidence that rule exists to prevent.
> `secret` is refused at every extension boundary outright. The result is a **monotone** system: every
> declaration a manifest can make is a restriction, so a hostile or mistaken manifest can make an
> extension harder to call but can never open a hole.

## Context

- ADR 0024's analysis is sound because the checker knows which `Core` parameters refuse a qualifier and
  which functions remove one. A Tier 1 extension currently has no way to say either, which leaves a gap
  precisely where extensions are most useful — a template engine, a query builder, an LDAP filter builder,
  a Markdown renderer are all *sinks*, and an extension that fetches or decodes remote data is a *source*.
- Without a rule, the default behaviour is not merely limited but unsound: if qualifiers were stripped on
  the way in and results came back plain, any `.mwlx` would be a universal bypass for ADR 0024. Passing a
  value through an extension would launder it.
- [ADR 0051](0051-standard-library-tiers.md) makes this concrete rather than hypothetical. Placing
  internationalization at Tier 1 means a `tainted` string from `Core\Request` is routinely handed to a
  **first-party** component and comes back formatted. If that round trip cleaned it, MWL's own stdlib would
  be the bypass.
- **This is time-sensitive.** M8 authors the `mwl:ext@1.0.0` WIT world and M9 freezes it; adding a
  qualifier axis afterwards is a breaking change to a published ABI. The plan already insists the WIT world
  be designed alongside the `Core` signatures so the two do not drift — this is that argument applied to
  the qualifier axis rather than the type axis.

## Decision

### 1. Contagion is the default and needs no declaration

An extension call is an operation under [ADR 0024](0024-taint-tracking-for-injection-sinks.md) § 2. If any
argument is `tainted`, every `string`/`bytes` in the result is `tainted`. This requires nothing in the
manifest and nothing new in the checker beyond treating an extension call like any other call.

### 2. Two declarable deviations, both restrictions

The manifest may say two things, and only these two:

- **A parameter refuses `tainted`.** The extension is a sink for that argument, and `mwl check` rejects a
  tainted operand at that position exactly as it does at a `Core\Db` query-text parameter. Declaring this
  can only make a call site fail that would otherwise have compiled.
- **A return is always `tainted`.** The extension is a source — it produces bytes MWL did not see enter.
  The result is tainted even when every argument was plain. Declaring this can only add a qualifier the
  caller must then launder.

```
// manifest / WIT-adjacent, illustrative
render:  func(template: string,          // refuses tainted - this is a sink
              data: list<string>)        // contagion applies, nothing declared
         -> string;

fetch:   func(url: string) -> tainted string;   // always a source
```

### 3. No extension may launder, and `secret` does not cross at all

There is **no manifest form** whose declared effect is `tainted string -> string`. ADR 0024 § 3 states that
the only way to remove the qualifier is a `Core` function whose contract names the single sink it is safe
for, plus the narrow, greppable `Core\Taint::assertTrusted` escape hatch. Neither is available to a guest.
A third-party HTML sanitizer can exist as an extension; what it cannot do is *assert* that its output is
safe for HTML. The caller launders with `Core\Html::escape` or takes responsibility explicitly at the call
site, where it is visible and greppable.

`secret` is refused at every extension boundary, in either direction. A `secret`-qualified value passed to
an extension is a compile-time diagnostic, and no manifest may declare a `secret` return. This is the same
refusal [ADR 0033](0033-secret-qualifier-for-confidential-values.md) already applies to serialize and to
isolate-crossing, for the same reason: the value is copied into memory whose subsequent handling MWL cannot
reason about. Where an extension genuinely must see a credential — a signing key for a protocol component —
`Core\Secret::reveal()` is the existing, deliberately conspicuous way to say so at the call site.

### 4. Monotonicity is the soundness property

Every declaration in § 2 is a *restriction*: it either rejects a call that would otherwise compile, or adds
a qualifier the caller must discharge. Nothing a manifest can say makes a program accept more.

This matters because a manifest is written by the extension's author, not by us. M9 pins hashes and
verifies signatures, but the analysis must not *depend* on that being done correctly. Under § 3's rule it
does not: a hostile manifest can make its own extension unusable, and cannot make a calling program less
safe than contagion alone would.

### 5. The checker applies these exactly as it does for `Core`

An extension call site is checked by the same code path as a `Core` call site, reading the qualifier from
the registered signature rather than from a table of built-ins. There is no second analysis and no
extension-specific relaxation, so the two cannot drift.

## Consequences

- **The WIT world gains a qualifier axis in M8**, before M9 freezes it. Two annotations, one on parameters
  and one on returns; a guest's generated bindings ignore both, since neither affects the wire
  representation — a `tainted string` is a `string`. The axis exists only in the manifest, for the
  compiler.
- **First-party extensions must declare correctly or be over-strict**, and over-strict is the safe failure.
  The intl component declares nothing at all and gets contagion, which is right: a formatted tainted number
  is still tainted, and reaches HTML output through ADR 0024 § 5's auto-escaping like any other value.
- **An extension cannot be a launderer, which is a real capability loss** and is not hidden here. A
  community HTML sanitizer cannot present itself as one. The alternatives are for the sanitizer to be
  adopted into `Core\Html` — where we own it, and where ADR 0024 § 3 already anticipates the roster growing
  — or for the caller to use `Core\Taint::assertTrusted` with a written reason. Both are visible; the
  rejected option is the invisible one.
- **`secret` never reaching an extension constrains protocol components.** A JWT signer as a `.mwlx` would
  need `Core\Secret::reveal()` at the call site. That is one reason
  [ADR 0060](0060-application-security-protocols.md) keeps the protocol roster in `Core` rather than at
  Tier 1.

## Alternatives rejected

- **No participation — qualifiers stripped at the boundary.** Nothing to design, nothing to freeze into the
  ABI. Rejected as unsound rather than merely limited: it makes every `.mwlx` a universal ADR 0024 bypass.
- **Sinks only, with no source declaration.** A simpler manifest and one less concept. Rejected: an
  extension that fetches remote data or decodes an untrusted file would return unqualified output from
  plain arguments, so untrusted bytes would enter the program clean — a hole exactly where Tier 1 is most
  valuable.
- **Allow a signature-verified, operator-trusted extension to declare a laundering function.** Would let a
  genuinely good third-party sanitizer exist as one. Rejected: it reopens ADR 0024 § 3's false-confidence
  hole behind a trust gate, and it breaks § 4's monotonicity, which is the property that lets the analysis
  stand without depending on signature verification having been configured correctly. "The operator
  installed it" is a much weaker guarantee than "the compiler team wrote it and named the one sink it is
  safe for."
- **Let `secret` cross with a declared, capability-gated parameter.** Would let protocol extensions hold
  keys. Rejected: it is the same argument ADR 0033 already heard and refused for serialization, and
  `Core\Secret::reveal()` already provides the escape hatch with the property that matters — it is at the
  call site, in the application's own source, and greppable.

## Verification

- **M8/M9:** fixtures for each rule. A tainted argument at a parameter declared to refuse it is a
  compile-time diagnostic; a plain argument to a function whose return is declared always-tainted yields a
  tainted value that a `Core\Db` query-text position then rejects; a tainted argument to an
  ordinary undeclared extension function yields a tainted result (contagion); a `secret` argument at any
  extension parameter is a diagnostic naming `Core\Secret::reveal()`.
- **M9:** a manifest attempting to declare a plain return where contagion would say tainted — the
  laundering form that § 3 says does not exist — fails manifest validation at load time with a diagnostic
  naming this ADR, rather than being silently ignored.
- **M9:** the adversarial extension suite gains a case asserting § 4 directly: an extension whose manifest
  makes the most permissive claims it is able to make still cannot cause a tainted value to reach a sink.
