# Handoff

## State

**ADR 0057's fold now reads four of § 1's five grammars.** `Grammar::Template`, `DateFormat`, `Regex` and
`Uri` each have their arm in `crates/nvs-types/src/intrinsics.rs:195-223`; `Duration` is the one row left
and is a one-line change behind its parser. Every arm reaches the **runtime's own** parser through a thin
`pub fn validate(&str) -> Result<(), String>` beside it — `nvs_stdlib::cldr:306`, `regex:581`, `uri:819` —
rather than through the parser itself, because a caller that discards the parse should not make `Piece`,
`Field` or `Compiled` public API. That is the standing decision's "one implementation, never a second
copy", and each entry point's own `# Errors` says what it shares with the throw it replaces.

**`nvs_stdlib::regex` gained a split, not a copy.** `compiled` was cache + build + `Fault`; the build half
is now `build(pattern, flags) -> Result<Compiled, String>` (`regex.rs:557`) and both the runtime path and
`validate` drive it. `uri::validate` runs **both** of `parse`'s throwing steps — `read` and `port_of` — for
the reason `tryParse`'s doc comment gives: folding only the grammar half is exactly the parser/validator
divergence that member exists to prevent.

**Four known gaps, and gap 3 is new and deliberate:** a *member's* restriction on a well-formed pattern is
left to run time. `Core\Time::parse` refuses a zonal field (`cldr::civil_fields_only`) because its zone is
argument 3; that is a rule about the member, not about the pattern language, and `Grammar` carries one
variant per language. Refusing it here needs a column § 1's table does not have. The other three gaps are
unchanged (whole-literal span, nothing prepared, no named/spread argument read).

**Six conformance cases moved to the runtime path**, the same repair
`str-format-refuses-every-mismatch.nvst` had last session: a malformed pattern written inline is now the
compile error, so each case binds it to a `string` first and keeps pinning the throw. The playbook bullet
under *Writing a test case* names all six and the cheap way to find the next set.

**`examples/intrinsics.nvs` already prints the stage's four lines** — `2026-08-28`, `matched`, `total: 42`,
`host=example.test` — and did before this session; the failing acceptance check was the missing test name,
not the example.

**Orientation gap, fourth session running:** the pack still does not print ADR 0057, which is this whole
goal stage. `[context] adrs` needs `0057 §§ 1, 3, 4`. This session did not re-read the ADR and worked from
the module docs' restatement of it instead, which is why gap 3 above is recorded there rather than argued
against § 1's own text.

## Next group

**The stage's two `.nvst` cases and the last grammar. Shared file set:**
`crates/nvs-types/src/intrinsics.rs`, `crates/nvs-types/tests/intrinsics.rs`, `crates/nvs-stdlib/src/time.rs`,
`tests/conformance/`.

- [ ] **`tests/conformance/core/a-literal-intrinsic-is-validated-while-checking.nvst`.** The half that
      *runs*: each folded member reached twice, literal and through a variable, printing the shared answer
      — `examples/intrinsics.nvs` is that program already, so this case is it with `--EXPECT--` around the
      four lines. Named by `docs/agent/loop-goal.toml:1333`.
- [ ] **`tests/conformance/reject/a-malformed-literal-intrinsic-is-a-compile-error.nvst`.** The other half:
      one malformed literal per grammar, `--EXPECTF-ERROR--`, whose indentation widens with the line number
      (conventions § *A `.nvst` test case*). `target/debug/nvs.exe check <scratch>` prints the exact text to
      freeze. Named by `docs/agent/loop-goal.toml:1334`.
- [ ] **The duration grammar.** ADR 0057 § 1 row 5, `Core\Time\Duration::parse`, argument 0. The arm is
      `crates/nvs-types/src/intrinsics.rs:223` and the runtime parser is behind
      `crates/nvs-stdlib/src/time.rs:476`'s `nvs_core_time_duration_parse` — give it the same `validate`
      entry point the other three got. No acceptance check names a test for it, so add one beside
      `crates/nvs-types/tests/intrinsics.rs:194`.

## Backlog

- Gap 2 — nothing is *prepared*; § 3's second effect needs a channel to `nvs-ir`
  (`crates/nvs-types/src/intrinsics.rs`'s module docs).
- Gap 1 — the diagnostic underlines the whole literal, not the offset inside it (same docs).
- ADR 0056 § 3's `[regex] backtracking = "deny"` — the tier a literal pattern landed in is decided while
  checking and still not reported (`crates/nvs-stdlib/src/regex.rs`'s module docs, gap 1).
- `Core\Time\Date::format` reads CLDR patterns and is deliberately not an intrinsic — § 1's asymmetry, one
  row per member whenever someone decides it has earned one.
- `[context] adrs` in `docs/agent/loop-goal.toml` needs `0057 §§ 1, 3, 4`.
