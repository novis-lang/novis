# Handoff

## State

**Stage 0 is open, and it outranks everything below.** [loop-goal.md](loop-goal.md) § *Stage 0* holds
items 1 to 17; **1 to 10, 16 and 17 are done**, so what is open is **items 11 to 15**, one named test
each at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py` short-circuits at stage
0, so **Stage 3 is shut until those five clear**. One item is a group — they share no files — and
they are ordered cheapest first, so take item 11.

**Item 10 landed**: `crates/mwl-runtime/tests/manifest_policy.rs` reads the workspace manifest's
`[profile.release]` block with its comments stripped and fails if `overflow-checks = true` is gone.
Nothing else was owed — the cast half is `[workspace.lints.clippy]` plus `verify.py`'s `-D warnings`,
and the setting's measurement is the manifest comment beside it. Only one slice was taken: item 11 is
a four-crate change sharing no file with item 10, so the second-slice test in the session prompt fails
on its file-set half, not on context.

**The plan's `Open now` said catch-up was finished and it was not.** That field, `Status`'s and
`On disk`'s counts now agree with the tree; the playbook carries the trap.

`benches/userland/` (`01-arith-loop.mwl`/`.php`) is **untracked**, left by an earlier session in this
run. It is item 15's `php_ratio` material — commit it with that item rather than sweeping it into an
unrelated slice.

Verify is green (1543 tests, 73 suites, clippy and fmt clean). Conformance **433**, differential 89.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, genuinely blocked behind ADR 0088's sink
carriers (`Core\Html\Markup`/`Cli\Text`); it is M4S work, not a slice to open ahead of the sinks.

## Next group — Stage 0 items 11 to 15, in order, starting at item 11

**These share no file set** — that is why one item is one group, and why the anchors below are per
item rather than per group. Item 15 is the largest by far (a second array representation with a
degrade path, ~300–500 lines across `array.rs` and the ABI); do not size the set from its first
member.

- [ ] **Item 11 — `secret == secret` lowers to the constant-time helper.** ADR 0033 § 5
      (`docs/adr/0033-secret-qualifier-for-confidential-values.md:203`) owns the rule and its ≈+8 ns.
      Closing test: `a_secret_equality_lowers_to_the_constant_time_helper`, `-p mwl-ir`. Anchors:
      `lower_binary` at `crates/mwl-ir/src/lower/expr.rs:2177` — the `Helper::Identical` arm at
      `:2201` and `Helper::NumericEq` at `:2241` are the two shapes to copy, and a plain `string` pair
      falls past both to the `BinOp` table at `:2268`, which `mwl-codegen` turns into `mwl_str_eq`;
      the `Helper` rows are `crates/mwl-ir/src/ir.rs:1372` and `crates/mwl-ir/src/print.rs:458`; the
      runtime address table is `crates/mwl-runtime/src/helpers.rs:1040`; a constant-time compare is
      already written and documented at `crates/mwl-stdlib/src/hash.rs:501` (`mwl_core_hash_equals`,
      doc at `:483`), but `mwl-runtime` cannot call `mwl-stdlib`, so the helper is a new one there.
      **Two things to settle before writing the arm**: (1) `mwl_ir::Ty` carries no qualifier, so
      `lower_binary`'s `lty`/`rty` cannot see `secret` — the checker has to record it, the way
      `mwl-ir` gap 12 already records a `Stringable` desugar the IR has no call expression for; and
      (2) `erase_checked_ty` (`crates/mwl-ir/src/lower/mod.rs:2197`) has **no** arm for
      `Ty::SecretString`/`SecretBytes`/`SecretTainted*` (`crates/mwl-types/src/ty.rs:53`), so they
      fall to its `_ => return None` — that is `mwl-ir` gap 11, and a scratch `.agent-tmp/*.mwl`
      declaring a `secret string` local will say in one run whether the erasure is owed first.
- [ ] **Item 12 — `$s as ?Uri` and `$s as ?Uuid` compile, `isValid` deleted from both.** ADR 0066
      §§ 1, 3 own the roster; `crates/mwl-stdlib/src/uri.rs`'s module doc owns the call-site
      consequence. Tests: `a_parse_roster_type_converts_nullably` and
      `a_class_type_still_refuses_the_nullable_conversion`, `-p mwl-types`.
- [ ] **Item 13 — `Core\Uri` compares by normalized components**, with a round-trip property and a
      differential corpus against the PHP 8.5 oracle, plus a `fuzz/fuzz_targets` entry beside
      `lex.rs`/`parse.rs`. `crates/mwl-stdlib/src/uri.rs`'s module doc owns the rule and both guards.
      Tests: `two_uris_compare_by_normalized_components`, `a_parsed_uri_round_trips_through_its_own_text`.
- [ ] **Item 14 — a call-stack limit rides the safepoint's emit site** (ADR 0020 § 1), then **item 15
      — a list-shaped array is packed** (`crates/mwl-runtime/src/array.rs`'s module doc). Read
      `Jit::new`'s stack-probe comment before 14: probes catch one oversized frame, this counts depth,
      and neither covers the other.

## Backlog

- Stage 4's counts are their own work: conformance 433 of 600, differential 89 of 150 — plan `Open now`.
- `Core\Out::capture` waits on ADR 0088's sink carriers — spec § 12, M4S.
- § 6 owes `decodeAs<T>` — `mwl_stdlib::json` gap 2, unblocked now a call site can write a type argument.
- ADR 0088's registry-wide qualifier classification for `Core` member rows — M4S.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
