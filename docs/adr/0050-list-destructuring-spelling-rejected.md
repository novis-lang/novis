# `rule:expressions/bracket-destructuring` — `list(...)` is rejected; `[...]` is the only destructuring spelling

- **Status:** Accepted
- **Date:** 2026-08-23
- **Scope:** the `list(...)` destructuring-target spelling (`Keyword::List`,
  `Parser::parse_destructure_from_list`) and its diagnostic. The `[...]` spelling, the element grammar both
  shared, and `list`'s status as a reserved word are all untouched.
- **Amends:** [`docs/spec/00-overview.md`](../spec/00-overview.md) § 3.3, which accepted `list(...)` as a
  second spelling of `[...]` and justified it by pointing at `<?php`'s acceptance as precedent.
  `rule:statements/require-is-the-only-inclusion-construct`'s framing paragraph cited the same pair as live
  precedent and is corrected there.
- **Amended by:** none.

> **In short:** `list($a, $b) = $pair;` no longer parses. It is a parse-time diagnostic (`E0230`) naming
> `[...]`, whose element grammar is identical in every position — key, nesting depth, skipped slot,
> reference marker. `list(...)` survived only on a justification `rule:statements/nvs-is-the-only-open-tag` has since deleted: that Novis keeps
> a duplicate PHP spelling when its meaning is the one Novis wants. That argument was always about keeping a
> spelling whose *meaning* had no other home; `[...]` already spells this meaning, so `list(...)` is the
> second spelling of a construct Novis fully covers, which is the exact trade this project has now refused
> four times.

## Context

- The spec accepted both spellings in one sentence, and the reason it gave was the precedent: "kept for the
  same pragmatic-superset reason `<?php` is kept as a second spelling of `<?nvs`."
  `rule:statements/nvs-is-the-only-open-tag` withdrew `<?php`. That left `list(...)`
  resting on a citation to a decision that no longer exists — not a weak argument, but no argument at all.
- The distinction that actually decides these cases was already available, in
  `rule:statements/require-is-the-only-inclusion-construct`: reuse a PHP spelling verbatim **when its existing
  meaning is exactly the one Novis wants**. `require` qualifies because nothing else in Novis spells "run this
  file in my frame, throw if it's missing." `list(...)` does not: `[...]` spells destructuring already, and
  the two produce the identical AST node from the identical element grammar. That is the same test
  `include`/`(int)$x`/`and`/`<?php`/`die` each failed.
- PHP's own history points the same way. `list()` predates the `[...]` short syntax by two decades; PHP 7.1
  added `[...]` precisely because `list()` reads as a function call and is not one, and modern PHP style
  guides prefer the bracket form. Novis is not preserving a distinction PHP maintains — it is preserving a
  spelling PHP itself moved away from.
- Nothing downstream distinguishes them. `parse_destructure_from_list` and `parse_stmt_maybe_destructure`
  both build the same `DestructureTarget` from the same `parse_destructure_elements`, so no type-checker or
  IR arm ever branched on which spelling produced a node.

## Decision

**`list(...)` as a destructuring target is rejected at parse time, naming `[...]`.**

```php
list(int $a, string $b) = $pair;    // E0230
[int $a, string $b]     = $pair;    // the replacement, identical in every element position
```

`nvs-syntax` still parses the whole construct through to its `;` — the target's elements, the `=`, and the
value expression — purely so the diagnostic can span the real statement and recovery can resume cleanly at
the next one. The parsed target is then discarded and the statement becomes `StmtKind::Error`: a rejected
construct never reaches the AST as a live node, exactly as `die` produces `ExprKind::Error` rather than
`ExprKind::Exit` under `rule:statements/exit-is-the-only-termination-keyword`.

`list` stays a reserved word. Freeing it would let a class or method be named `list`, which is a separate
question this ADR does not open, and keeping it reserved is what lets the diagnostic fire at all.

## Consequences

**Positive**

- One destructuring spelling instead of two, with no behavioural question behind the choice — the same
  narrowing ADRs 0021, 0034, 0045 and 0049 each already made. The rule a reader has to learn is `[...]`, and
  the language has one less exception to it.
- The `[...]`-versus-array-literal backtracking in `parse_stmt_maybe_destructure` is now the *only* path
  into a destructuring statement, so the grammar has one entry point rather than two.

**Negative**

- **One more line item for `nvs convert`'s (M11) rewrite pass**: `list(a, b) = c;` → `[a, b] = c;`. It is a
  bracket-for-parenthesis substitution over a construct whose element grammar is already identical, so it is
  the cheapest rewrite the converter has been given yet — cheaper than ADR 0034's `(int)$x` → `$x as int`,
  which at least has to identify an operand.
- **A further, small subtraction from the "pragmatic superset" promise**, in the same vein as ADRs 0034,
  0045 and 0049. A PHP file using `list()` anywhere no longer parses unconverted. Mitigated by the fact
  that Novis already requires every destructuring leaf to carry a type, so no real PHP `list()` call site
  parses unedited regardless of which bracket it uses.

## Alternatives rejected

- **Keep `list(...)`, rewrite § 3.3's justification.** The status quo with honest reasoning. Rejected: once
  the citation is removed there is nothing left to write in its place — every argument for keeping it
  ("PHP programmers know it") applies verbatim to `<?php`, `die`, `and`, and `(int)$x`, all rejected. A
  reason that proves too much is not a reason.
- **Keep `list(...)` only for the no-key, no-nesting shape.** Rejected: a spelling that works for
  `list($a, $b)` but not `list('id' => $id)` is a third rule to teach, worse than either uniform answer.
- **Free `list` as an identifier at the same time.** Deliberately not bundled: it is a lexer/reserved-word
  question with its own compatibility surface (a PHP class or method named `list`), unrelated to which
  destructuring spelling survives. Reserved is also what makes this diagnostic possible.

## Verification

- `crates/nvs-syntax/src/parser.rs`'s `list_is_diagnosed_naming_bracket_destructuring` test asserts `E0230`
  and a `StmtKind::Error` for the positional, skipped-slot, and string-keyed shapes, and asserts the `[...]`
  spelling of each of the same three is still accepted as a live `StmtKind::Destructure`.
- The retired `list_is_a_second_spelling_of_bracket_destructuring` and
  `list_elements_still_require_a_type_like_brackets_do` tests are replaced by that one — the second is
  subsumed, since an untyped `list($a, $b)` now trips this diagnostic as well as the missing-type one.
- No `nvs-hir`, `nvs-types` or `nvs-ir` change was needed: neither crate ever saw which spelling produced a
  `DestructureTarget`.
