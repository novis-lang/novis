# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 13, 16 and 17 are done**, so what is open is **items 14 and 15**, three
or four named tests each at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py`
short-circuits at stage 0, so **Stage 3 is shut until those two clear**.

**Item 13 landed as `$uri->compareTo($other)`, not as `==`.** ADR 0090 § 4 fixes `==` on two objects
as identity and closes the door on a per-class equality hook, and it names `compareTo(...) == 0` as
the spelling for content equality — so `Core\Uri` satisfies `Comparable` by member, the way
`Core\Time\Instant` does. The normalization is the whole of RFC 3986 § 6.2.2 and stops short of
§ 6.2.3, which `crates/mwl-stdlib/src/uri.rs`'s module doc § *Comparison normalizes; `parse` still
reports* owns along with why the two rules do not collide. `loop-goal.toml`'s item-13 comment said
`==` and has been corrected; the playbook's new *Tooling* bullet says why that mattered.

Verify is green (1551 tests, 73 suites, clippy and fmt clean); valgrind clean on the `compareTo`
probe, which retains nothing — every slot read is `instance::slot`'s borrow. Conformance **435**,
differential **90**. The new `fuzz/fuzz_targets/uri.rs` type-checks on stable (`cargo check
--manifest-path fuzz/Cargo.toml --bin uri`); it has never been *run*, libFuzzer being unavailable on
Windows.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, genuinely blocked behind ADR 0088's sink
carriers; it is M4S work, not a slice to open ahead of the sinks.

## Next group — item 14, then 15

**These two share no file with each other**, so a session takes exactly one and stops; the second
slice the ceiling allows has nothing cheap to be. Item 15 is by far the larger of the two.

- [ ] **Item 14 — a call-stack limit rides the safepoint's emit site** ([ADR 0020 § 1](../adr/0020-error-escalation-ladder.md)
      owns the mechanism, the 8 MB ceiling, the two tiers and the measured cost). Tests:
      `a_function_entry_checks_the_stack_limit`, `a_leaf_function_under_the_slack_emits_no_stack_check`,
      `a_runaway_recursion_reports_a_limit_rather_than_faulting`, `-p mwl-codegen`. Anchors:
      `crates/mwl-runtime/src/ctx.rs:103` (the hot `safepoint` field the `stack_limit` sits beside, and
      the `SAFEPOINT_OFFSET` the module doc's offset table names), `crates/mwl-codegen/src/emit.rs:758`
      (`emit_safepoint`, where the compare goes), `crates/mwl-ir/src/lower/mod.rs:1506` (the IR side's
      `emit_safepoint`, which lowers to nothing today — `mwl-ir` gap 14). Expect a one-time step in
      `benches/abi-probe`'s `frame_depth`; say so in the commit.
- [ ] **Item 15 — a list-shaped array is packed**, with a degrade path
      ([`array.rs`](../../crates/mwl-runtime/src/array.rs)'s module doc owns the decision and what the
      ABI addition costs if it waits). Tests: `a_list_shaped_array_holds_no_index_map`,
      `an_integer_subscript_allocates_no_key`, `a_non_sequential_key_degrades_the_packed_array`,
      `both_representations_answer_every_primitive_alike`, `-p mwl-runtime`. Anchors:
      `crates/mwl-runtime/src/array.rs:327` (`ArrayHeader`, the layout that grows a second form).
      ADR 0007 § 5 is **unchanged** — representation, not semantics. It also owes the first
      `docs/perf/history.ndjson` entry with a `php_ratio` ([ADR 0026](../adr/0026-performance-measurement-methodology.md));
      `benches/userland` holds the twenty-program suite that produces it.

## Backlog

- `Core\Json::decodeAs<T>` — spec § 6's last member (`mwl_stdlib::json` gap 2, now unblocked).
- `Core\Out::capture` — the one key left in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`;
  waits on ADR 0088's sink carriers.
- `Uri`'s two decoders answer `string` where the octets may not be UTF-8 (`uri.rs` gap 2) — a spec
  slice, now that `Tag::Bytes` exists.
- ADR 0088's registry-wide qualifier classification for `mwl-stdlib` member rows — lands with M4S.
- `do`/`while` is the one M4 control-flow statement that does not lower (`mwl-ir` gap 1's remainder).
- `docs/spec/02-php-migration.md` is 31% classified; `python tools/check-migration.py` reports it.
