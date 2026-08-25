# Handoff

## State

**The owned-temporaries stack exists, and the throwing edge no longer leaks what an expression was
holding.** `mwl_ir::lower::Lowering::owned_temporaries` is the frame-level counterpart of `Env` for
references that have no name; `landing_block` releases the whole stack on **both** exits, which is the one
thing the propagate/catch asymmetry does not reach. Staged today: a call's arguments and receiver
(`account_for_arg`), and the operands and partial results of `.`, an interpolation and an `echo`. Measured:
`.agent-tmp/throw-temp-leak.mwl` lost 32 bytes and `throw-temp-leak2.mwl` 59 in 3 blocks before the change,
both green after — `tools/leak-check.sh`, plus `examples/core.mwl`/`text.mwl`/`report.mwl` still clean.

Two gaps of the same family stay open and are named where they live: `owned_temporaries`' own field doc
holds the one shaped differently (an argument being **transferred** when a *later* argument throws, which
wants a second entry kind released on the error edge and forgotten on the normal one, plus one forget at
each of the three transferring call sites in `expr.rs`), and `landing_block`'s *Known gap* holds the
producers that still release inline — a normalized subscript key, a `match` subject. `mwl-ir`'s crate doc
gap 2 is the index of both.

The ratchet (`crates/mwl-stdlib/tests/spec-members-outstanding.txt`) is unchanged at **7 keys**; conformance
is 412 of 600 and differential 89 of 150. `examples/collect.mwl`'s frontier is still `Core\Out::capture` at
`collect.mwl:47`, which lands with M4S's sink work (ADR 0088 §§ 3, 5).

## Next group — the last three §§ 4–5 ratchet keys, in three slices

[2] and [3] share `crates/mwl-stdlib/src/regex.rs` and are one design landed twice; [1] is a single row in
`crates/mwl-stdlib/src/time.rs` and rides along with either.

- [ ] **`withTime`** — spec row at `docs/spec/01-core-library.md:491`, `$d->withTime(TimeOfDay $t):
      DateTime`, "the common half of `with`, spelled as the operation it is". `time.rs:943`/`:1264`/`:1397`
      are the three `with` rows to mirror, `time.rs:978`/`:985` are `date`/`timeOfDay` answering with the
      component types, and `time.rs:279` is the `address()` arm a miss of which is a runtime panic. The
      helper is `with`'s body reached from a `TimeOfDay` instead of an options bag. Strike `§4 withTime`
      from the ratchet in the same commit; § 4 is then whole.
- [ ] **`Core\Regex\Pattern` and `compile`** — `regex.rs:63` states gap 1 (both rows need `Pattern`),
      `regex.rs:112` is `CLASS`, `regex.rs:171` is how `Core\Regex\Match` names a second class in this
      module, and `regex.rs:294` is `address()`. Spec § 5's rows say what a compiled pattern answers.
- [ ] **`replaceWith`** — the same file and the same `Pattern`; it is the callback half, so
      `registry::CoreTy`'s closure parameter shape is what it adds over `compile`.

Each of the three owes the five things `playbook.md` § *Adding a `Core` member* lists, the fifth being the
ratchet line.

## Backlog

- `Core\Arr::from` — § 2's last row, waiting on an `Iterable`/`Iterator` argument (`arr.rs` gap 1).
- The transferred-argument window on a throwing edge — `mwl_ir::lower::Lowering::owned_temporaries`' doc.
- `Core\Json::decodeAs<T>` — `json` gap 2; the written type argument it waited on exists now.
- `Core\Heap` and the `Iterable` its three rows declare — spec § 9.
- `Core\Out::capture` — `examples/collect.mwl:47`'s frontier, lands with ADR 0088 §§ 3, 5.
- ADR 0057's intrinsic pattern folding — `mwl_stdlib::cldr` gap 1, now three `format` members deep.

## Orientation gaps, still open in `docs/agent/loop-goal.toml`

- `[context] modules` names four `mwl-runtime` files but not `src/array.rs`, whose
  `next_slot`/`key_at`/`value_at`/`set` contract every `Core\Arr` slice reads.
- `[context] modules`' `mwl-stdlib` patterns cover `str`/`math`/`hash`/`encoding`/`registry`/`lib` but not
  `src/time.rs`, `src/cldr.rs` or `src/regex.rs` — the next group's whole file set.
- `[context] modules` names no `mwl-ir` file at all, so a session working `lower/` gets no map line for
  `src/lib.rs`'s known-gap list, which is the index this session's whole work is filed under.
