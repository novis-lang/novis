# Handoff

## State

**§ 4's component types are built; `withTime` is all that is left of the section.**
`Core\Time\Date` and `Core\Time\TimeOfDay` are registered classes with the
`at`/`format`/`plus`/`minus`/`with`/`compareTo` shape § 4 writes, and `$d->date()` /
`$d->timeOfDay()` answer with them. The ratchet
(`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is at **7 keys**, of which **1 is § 4**;
conformance is **412** of 600 and differential **89** of 150.

Three things these two slices settled, recorded where the work lives:

- **There is no `Core\Month`.** The spec writes no member that takes or answers with one — `Core\Weekday`
  exists only because `$d->weekday()` does — so an enum nothing names was surface with no spec home. Two
  earlier notes claimed § 4 owed it and that `registry::ENUMS` said so; neither the enum nor that sentence
  was ever on disk. `mwl_stdlib::time`'s gap 1 owns the correction.
- **A pattern is refused rather than filled in.** `Date::format` throws on a time-of-day or zonal letter and
  `TimeOfDay::format` throws on a calendar or zonal one; `crate::cldr::date_fields_only` /
  `time_fields_only` own why (an invented hour is a wrong answer stated confidently).
- **The two step members are exact mirrors.** `Date::plus` refuses a unit smaller than `Unit::Day`;
  `TimeOfDay::plus` refuses one of `Unit::Day` or larger and **wraps** at midnight, since a clock reading
  has no date to carry into. Both helpers' doc comments state it.

`examples/collect.mwl`'s frontier is unchanged: `Core\Out::capture` at `collect.mwl:47`, which lands with
M4S's sink work (ADR 0088 §§ 3, 5).

**Valgrind: clean over the new members, and the one red run is inherited.** `leak-check.sh` on a probe
that *catches* a throw from any `Core` member taking a `string` loses that argument — `mwl-ir`'s recorded
gap at `crates/mwl-ir/src/lower/call.rs:432`, reproduced on `Core\Encoding::fromHex` with no `Core\Time`
in sight. A playbook bullet now says how to recognize it in one look instead of bisecting.

**Orientation gaps, both still open in `docs/agent/loop-goal.toml`:**

- `[context] modules` names four `mwl-runtime` files but not `src/array.rs`, whose
  `next_slot`/`key_at`/`value_at`/`set` contract every `Core\Arr` slice reads.
- `[context] modules`' `mwl-stdlib` patterns cover `str`/`math`/`hash`/`encoding`/`registry`/`lib` but not
  `src/time.rs` or `src/cldr.rs` — this session's whole file set, and the second session running to find
  out.

## Next group — the leak on a throwing edge, then § 4's last row, in three slices

The first two share `crates/mwl-ir/src/lower/`; the third is back in `time.rs` and is small enough to ride
along with either.

**Shared file set:** `crates/mwl-ir/src/lower/call.rs` (`:432` `release_call_temporaries` and its gap note,
`:265` where a borrowed argument becomes a temporary, `:394` the same gap named from the other side), `crates/mwl-ir/src/lower/expr.rs` (`:266` and `:1668`, the two
`landing_block` sites an owned-temporaries stack has to reach), `crates/mwl-ir/src/lower/exception.rs`
(`:39` the `Throw` terminator, `:88` the landing-block folding).

- [ ] **Release call temporaries on the throwing edge** — `call.rs:432` states the gap and the fix in one
      sentence: the owned-temporaries stack `docs/agent/loop-goal.md` already names, with
      `release_call_temporaries` as one more caller rather than a second design. Reproduce with
      `.agent-tmp/throw-probe5.mwl`-shaped source (a `try` around `Core\Encoding::fromHex("zzzz…")`) under
      `tools/leak-check.sh`; the lost size is `16 + strlen`.
- [ ] **The same stack for `landing_block`'s own gap** — `call.rs:432` says the two are one gap seen twice,
      so close the second in the same design and strike both notes.
- [ ] **`withTime`** — spec row at `docs/spec/01-core-library.md:491`, `$d->withTime(TimeOfDay $t):
      DateTime`. A `DATETIME` instance row (`time.rs:985`, beside `timeOfDay`), an arm in
      `datetime_address` (`time.rs:1050`), and a helper that rebuilds `zoned_of(...)`'s date at
      `clock_of(args, 1, …)` in the receiver's own zone. Then strike `§4 withTime` from the ratchet and
      § 4 is whole.

## Backlog

- `Core\Arr::from` — § 2's last row, waiting on an `Iterable`/`Iterator` argument (`arr.rs` gap 1).
- `Core\Regex::compile`/`replaceWith` — both need `Pattern` (`mwl_stdlib::regex` gap 1).
- `Core\Json::decodeAs<T>` — `json` gap 2; the written type argument it waited on exists now.
- `Core\Heap` and the `Iterable` its three rows declare — spec § 9.
- `Core\Out::capture` — `examples/collect.mwl:47`'s frontier, lands with ADR 0088 §§ 3, 5.
- ADR 0057's intrinsic pattern folding — `mwl_stdlib::cldr` gap 1, now three `format` members deep.
