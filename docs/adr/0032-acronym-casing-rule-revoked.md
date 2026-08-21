# ADR 0032 — The acronym-as-one-word casing rule is revoked; only an identifier's leading character is checked

- **Status:** Accepted
- **Date:** 2026-08-21
- **Scope:** revokes [ADR 0029](0029-identifier-casing-is-checked.md) § 1 ("Acronyms are one word, never
  kept all-caps") in full, for every casing-checked category (`PascalCase` and `camelCase` alike). The
  casing checker enforces exactly the table in ADR 0029's *Decision* section — an identifier's first
  character has the case its category requires, and everything after that is merely alphanumeric ASCII —
  with no further restriction on internal capitalization. `HTTPClient`, `IOStream`, `parseHTTPRequest` and
  any other run of consecutive uppercase letters are accepted again, on equal footing with the
  one-word-acronym spellings (`HttpClient`, `IoStream`, `parseHttpRequest`) ADR 0029 § 1 previously required
  instead. Every other rule ADR 0029/0030 states — the leading-character requirement itself, the
  zero-suppression stance, the no-leading-underscore rule, the targeted `__construct` diagnostic — is
  untouched.
- **Amends:** [0029](0029-identifier-casing-is-checked.md) § 1 — revoked outright, along with the
  corresponding "all-caps acronym" entries in that ADR's *Diagnostics* and *Verification* sections, which
  now produce no diagnostic instead of one.
- **Amended by:** none.
- **Relates to:** [0030](0030-no-leading-underscores-constructor-spelling.md) — a separate amendment to
  ADR 0029, untouched by this one; the leading-underscore and `constructor`-spelling rules stand exactly as
  it left them.

> **In short:** ADR 0029 § 1's "acronyms are one word, never kept all-caps" rule — implemented in
> `crates/mwl-syntax/src/casing.rs` as a ban on any run of two-or-more consecutive uppercase letters — is
> revoked. The casing checker now inspects only an identifier's first character (does it have the case its
> category requires?) plus the requirement that the rest is alphanumeric — exactly what ADR 0029's own
> regex table already stated, with nothing layered on top of it. `HTTPClient` is accepted again,
> indistinguishable from `HttpClient`.

## Context

- ADR 0029 § 1 asked for every acronym to be spelled as a single capitalized word (`HttpClient`, never
  `HTTPClient`), reasoning that recognizing an acronym without a maintained dictionary is impossible, so
  the compiler instead bans every acronym-preserving spelling outright.
- The literal regex table in ADR 0029's *Decision* section (`^[A-Z][A-Za-z0-9]*$` for `PascalCase`,
  `^[a-z][A-Za-z0-9]*$` for `camelCase`) does not by itself express that rule — `HTTPClient` matches either
  pattern exactly as well as `HttpClient` does. Implementing § 1 as written therefore required a second,
  separate check (a ban on consecutive uppercase letters) layered on top of the table, which is what
  `crates/mwl-syntax/src/casing.rs` originally shipped with.
- That second check costs more than the table's plain pattern promises: it rejects a deliberately-spelled,
  widely recognized acronym (`HTTP`, `IO`, `XML`, `ID`, …) used anywhere in an identifier, with no escape
  hatch at all — a false positive on ordinary, unsurprising spellings that ADR 0029 § 3 itself already
  argues MWL should avoid manufacturing.
- Revisited immediately, in the same development pass that first implemented the check, before any stdlib
  or userland code could depend on the stricter rule — the cheapest possible time to reverse it, per ADR
  0029's own "decided now, while the standard library is still unwritten" reasoning.

## Decision

1. `crates/mwl-syntax/src/casing.rs`'s `is_pascal_case`/`is_camel_case` check exactly two things: the
   identifier's first character has the case its category requires, and every remaining character is
   ASCII alphanumeric. No run-length, acronym, or "one capitalized word per acronym" check of any kind.
2. ADR 0029 § 1 is revoked in full. Its own examples (`HttpClient`, `IoStream`, `XmlParser`,
   `parseXmlPayload`, `httpStatus`) remain valid spellings — they always were, under the table's plain
   pattern — but so now are `HTTPClient`, `IOStream`, `XMLParser`, `parseXMLPayload`. Nothing in the
   checker distinguishes an acronym from any other run of letters.
3. The mechanical rename suggestion the checker attaches to an actual violation (wrong leading case, a
   leading underscore, `SCREAMING_SNAKE_CASE` violated, …) still normalizes a consecutive-uppercase run into
   one capitalized word when it recomputes a suggested name — that word-splitting behavior is unaffected by
   this decision, and stays because it produces one plausible suggestion, not because all-caps runs are
   forbidden. It was never, and still isn't, part of the acceptance check itself.

## Diagnostics

No diagnostic changes. `E0110`/`E0111`/`E0112`/`E0113`'s messages are unchanged; they simply no longer fire
for an all-caps acronym, because that spelling was never actually disallowed by anything except the rule
this ADR revokes.

## Consequences

**Positive**

- The checker matches its own documented regex table exactly, with no second, table-external heuristic a
  reader has to separately learn.
- No false positive for a deliberately-spelled, widely recognized acronym used as, or inside, an
  identifier — `HTTPClient`, `parseHTTPRequest`, and similar spellings compile without a rename.
- One less mechanical rewrite for `mwl convert` to perform when porting an existing PHP codebase: a PHP
  class already spelled `DOMDocument`-style converts unchanged instead of needing acronym-flattening.

**Negative**

- MWL loses the one-spelling-per-acronym guarantee ADR 0029 § 1 wanted specifically: `HTTPClient` and
  `HttpClient` are now both valid, distinct names, reopening the narrow "two house styles in one codebase"
  concern ADR 0029's own *Consequences* section otherwise argues the whole casing check exists to close.
  Accepted here as the smaller cost against the false-positive/no-escape-hatch cost above.
- Every other zero-exception guarantee ADR 0029/0030 makes — casing enforced by category, no leading
  underscore, no suppression mechanism — is untouched; only this one, narrowly-scoped acronym rule is given
  up.

## Alternatives rejected

- **Keep the consecutive-uppercase ban, but allow exactly two-letter runs** (so `IO`/`ID` pass but
  `HTTP`/`XML` still fail). Rejected: an arbitrary length cutoff with no principled basis that still fails
  for any acronym three letters or longer.
- **A maintained acronym allowlist** (`HTTP`, `XML`, `IO`, … recognized; anything else rejected). Rejected
  for the same reason ADR 0029 § 1 itself already rejected a dictionary: upkeep, and still ambiguous the
  moment two recognized acronyms sit next to each other (`HTTPXMLParser`).
- **Warn instead of silently accepting.** Rejected: MWL's casing check has no warning severity or
  suppression mechanism anywhere else ([ADR 0029](0029-identifier-casing-is-checked.md) § 3); introducing
  one just for this one rule would be a bigger precedent than the rule itself.

## Verification

- `crates/mwl-syntax/src/casing.rs`'s unit test that previously asserted an all-caps acronym (`HTTPClient`)
  produces `E_BAD_TYPE_CASING` is replaced with `an_all_caps_acronym_is_accepted`, asserting no diagnostic
  at all, plus a parallel `camelCase` case (`an_all_caps_acronym_in_a_camel_case_name_is_accepted`,
  `parseHTTPRequest`).
