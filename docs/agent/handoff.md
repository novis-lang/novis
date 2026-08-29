# Handoff

## State

**ADR 0057's intrinsic folding is half built.** `crates/nvs-types/src/intrinsics.rs` holds § 1's table
verbatim — six rows, matched nominally against the *declaring* class — and `check_call` is consulted from
both call sites in `crates/nvs-types/src/expr/calls.rs` (`infer_method_call:111`, `infer_static_call:243`)
once the target is resolved and the arguments are typed. Only `Grammar::Template` reads its literal so
far; the other four arms are the next group and are a one-line change each behind their parser.

**`Core\Str::format` is the worked example of § 4's soundness rule.** `nvs_stdlib::format::Pieces` is now
the one walk over the `printf` grammar: `format` drives it to render, `placeholders` drives it to describe,
so a template the checker refuses is exactly one the runtime would have thrown on — asserted by agreement
in `crates/nvs-stdlib/src/format.rs`'s `a_prepared_literal_and_its_runtime_twin_share_one_implementation`.
Two codes: `E0769` for a literal the grammar refuses, `E0770` for a template its argument list does not fit.
Next free is `E0771`.

**One conformance case moved to the runtime path.**
`tests/conformance/core/str-format-refuses-every-mismatch.nvst` now reads each template out of a variable,
which is ADR 0057 § 2's division and keeps both halves of § 4's one implementation under test — the literal
forms of its four mismatches are compile errors and can no longer reach a `catch`.

**Nothing is prepared, only validated** — § 3's second effect needs a channel to `nvs-ir` and is gap 2 in
that module's docs, beside the two other gaps (the diagnostic underlines the whole literal, and a named or
spread argument is skipped).

**Orientation gap, third session running:** the pack still does not print ADR 0057, which is this whole
goal stage. `[context] adrs` needs `0057 §§ 1, 3, 4`; reading the ADR by hand cost ~2.5k of context that a
selector would have made free.

## Next group

**The three remaining grammars, all in one file set:** `crates/nvs-types/src/intrinsics.rs:181` (the
`Grammar::Regex | Uri | DateFormat | Duration` arm of `check_call`), plus one `pub` entry point per parser
and one case each in `crates/nvs-types/tests/intrinsics.rs`. Each is the shape `check_template` already
has — fold, hand the text to the runtime's own parser, report `E0769` with its message.

- [ ] **The date pattern.** ADR 0057 § 1, rows 3 and 4. `nvs_stdlib::cldr::compile`
      (`crates/nvs-stdlib/src/cldr.rs:218`) already answers `Result<Vec<Piece>, String>` and is
      `pub(crate)`: make it `pub`, make `mod cldr` public in `crates/nvs-stdlib/src/lib.rs:201`, and add
      the `Grammar::DateFormat` arm. `Core\Time::parse`'s pattern is argument **1** and the row already
      says so. Test: `a_literal_date_format_is_validated_while_checking`.
- [ ] **`Core\Regex`'s pattern.** § 1 row 1 over `crates/nvs-stdlib/src/regex.rs:516`'s `compiled`, which
      takes flags and a member name and answers a `Fault` — it needs a pattern-only entry point beside it
      rather than a second call shape, and ADR 0056 § 3's *tier* is what it should answer with eventually
      (gap 1 in that module's docs). Test: `a_literal_regex_pattern_is_prepared_while_checking`.
- [ ] **The URI.** § 1 row 2 over `crates/nvs-stdlib/src/uri.rs:1439`'s `nvs_core_uri_parse`, whose
      well-formedness check is inside the helper body and has to come out into a `pub fn` first. Test:
      `a_literal_uri_is_validated_while_checking`.
- [ ] **`examples/intrinsics.nvs`.** The stage's last check wants `2026-08-28`, `matched`, `total: 42`,
      `host=example.test` — one program over all four grammars, written after they land.

## Backlog

- `Core\Time\Duration::parse`'s row is on the list and unread; ADR 0070's grammar is `nvs_syntax::duration`.
- Preparation itself (§ 3's second effect) — the artifact-cache channel to `nvs-ir`, `intrinsics.rs` gap 2.
- The exact offset inside a literal, which needs a position-recording decoder in `crate::string_lit`.
- A named argument to an intrinsic folds nothing — `intrinsics.rs` gap 3, `links.rs` gap 1's twin.
- M4 item 15's seventeen `nvs-ir` lowering refusals stand; ratchet at `crates/nvs-ir/tests/refusals.rs:66`.
- The 1000-case conformance corpus is M4's residue, met as orders 1-4 grow the suite.
