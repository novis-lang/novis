# Handoff

## State

**Stage 8's corpus count is 994 of 1000, and the `Qual` gap the last two sessions recorded is closed
end to end.** A registry row's ADR 0088 § 2 classification now reaches the checker
(`MethodSig::param_quals`, filled in `nvs_types::core_lib::method_sig`), the call check admits a
`tainted` argument at a `Contagious`, `Neutral` or `Launder` parameter, and a `Contagious` call's
answer carries the qualifier back out — through the atom itself, an array's element and every member
of a union, so a `?string` or an `array<string>` return does not launder. Where the result has
nowhere to carry the bit, the tainted argument is **still refused**: that conservative half is
deliberate and `expr::quals::admits_tainted_argument`'s doc comment owns why.

**Only the `tainted` axis moved.** `secret` is refused at exactly the positions it was refused at
before; whether a `Neutral` parameter launders `secret` is still ADR 0088's question to answer, and
nothing in this tree now depends on the answer.

**Two new gaps, both narrow, both recorded where the code is.** A **generic** `Core` member
(`Core\Json::decodeAs<T>`) returns from `check_generic_args` before the admission loop, so it still
refuses a tainted argument; and a `...` spread carries its qualifier on the array rather than on the
entries, which the admission deliberately does not ask about
(`crates/nvs-types/src/expr/args.rs:@check_args_typed`).

**Three known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer and not of the argument (`crates/nvs-runtime/src/graph.rs` § *Known
gaps*); item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).

## Next group

**Spend the closed gap on corpus cases — the acceptance check wants 1000 and the tree is at 994.**
One file set: `tests/conformance/core/`, `tests/conformance/reject/` and
`crates/nvs-types/tests/tainted.rs`, with the rows read from `crates/nvs-stdlib/src/str.rs` and
`crates/nvs-stdlib/src/regex.rs`. Nothing below needs a checker change.

- [ ] **A union and an array result keep the qualifier** (ADR 0088 § 2). `Core\Str::after` returns
      `?string` (`crates/nvs-stdlib/src/str.rs:319`) and `Core\Str::split` an `array<string>`
      (`:341`); both are contagious, so a tainted subject makes each answer tainted through
      `crates/nvs-types/src/expr/quals.rs:@tainted_result`. Assert the *refusal* of the plain
      declared type, which is what proves nothing laundered.
- [ ] **A `Sink` still refuses after the admission landed** (ADR 0088 § 1) — `Core\Regex::compile`'s
      pattern (`crates/nvs-stdlib/src/regex.rs:106`) is the one to ask, as a `reject/` case beside
      `bytes-pack-and-unpack-refuse-a-tainted-format.nvst`. Note `compile` is also an
      `crate::intrinsics` row, so check which diagnostic actually lands before freezing the text.
- [ ] **Every classification a row can write, asserted by counting** — the four marks over the whole
      registry, in `crates/nvs-types/tests/tainted.rs`, so a member added later without a mark shows
      up as a count rather than as nothing. `nvs_types::core_lib`'s
      `every_registered_parameter_carries_its_classification_into_its_signature` is the sweep to
      copy.

## Backlog

- A generic `Core` member does not reach the admission — `crates/nvs-types/src/expr/args.rs`.
- `Neutral` and `secret`: the laundering decision ADR 0088 owes before that half can be written.
- Item 18: `Core\Secret::reveal()` is not in the registry — `nvs_types::expr::quals`.
- Item 22: `Core\Script`'s members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Live::admit` asks its same-class check of the answer — `crates/nvs-runtime/src/graph.rs`.
- `gaps.py --errors`'s last row is `crates/nvs-stdlib/src/csv.rs:610`, unreachable from source.
