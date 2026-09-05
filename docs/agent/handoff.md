# Handoff

## State

**Stage 3's `nvs check` check is closed.** `nvs check` resolves the configuration tree exactly as
`nvs run` does — ADR 0103 § 1's roots, § 3's later-wins, ADR 0104 § 2's `[[app]]` fold for the entry
— and hands the folded `[capabilities]` to `nvs_types::check_program_granted`, so ADR 0067 § 10's
literal `Core\Db::open` host is answered while compiling. A `nvs.toml` that does not resolve fails
the check as that configuration error, before the program is parsed. `nvs_types::intrinsics`' gap 6
— "checking has no configuration in front of it" — is answered for this command.

**`nvs run` deliberately still checks with no grants**, and `front_end_granted`'s doc comment in
`crates/nvs-cli/src/main.rs` is that rule's home: at run time the refusal is
`nvs_runtime::capability::require`'s and ADR 0118 § 5 makes it catchable, so hoisting it into `run`
would turn a catchable denial into a refusal to start. `nvs check` is allowed to be the stricter of
the two because § 10 is titled for it. Nothing is blocked on a decision.

**Spec § 13's `Core\Router` row now names every parameter** (ADR 0063 R2): `match`'s `$method` and
`$path`, `methodsFor`'s `$path` and `url`'s `$params`, all four agreeing with the registry's `names`
— which `every_registry_rows_names_are_the_specs_signature_column` holds. That row is in § 13,
*Compiler-facing surfaces*; a reference calling it "§ 15's row" is stale.

## Next group

**ADR 0116 § 2's sweep, widened to what an array holds.** One file set:
`crates/nvs-runtime/src/release.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-runtime/src/array.rs`, `examples/cycles.nvs`.

- [ ] **The unreachability tally counts an object held only by an array element.** The goal's
      § *Standing decisions* item 7 bounds it: the walk may only ever move an object from "left
      alone" to "shown unreachable", and an element edge it cannot prove is an external hold that
      leaves the object exactly as today, pinned by a `debug_assertions` assertion —
      `crates/nvs-runtime/src/object.rs:1658`, `crates/nvs-runtime/src/release.rs:96`.
- [ ] **`examples/cycles.nvs` gains an array-closed cycle**, so the WSL valgrind leg is what proves
      the widening rather than a unit test standing in for it (`docs/agent/commands.md` owns the
      run) — `examples/cycles.nvs:1`, `crates/nvs-runtime/src/array.rs:1`.
- [ ] **Goal 27's stage 3 item 4 is answered before that goal starts.** It asks whether `router.rs`
      gap 3 ("`Core\Router::match` is absent") or the plan is wrong; the member landed, so the item
      is a settled question rather than a judgement —
      `docs/agent/goals/27-gap-owners.md:70`, `docs/agent/goals/27-gap-owners.handoff.md:45`.

## Backlog

- **ADR 0102 § 1 gains the sentence saying where a capture is decoded**, once a goal's standing
  decisions admit that ADR — `docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:96`,
  `crates/nvs-stdlib/src/router.rs:1007`.
- The eight spec §§ 16-17 classes still have no owner on the chain — `docs/agent/carried-gaps.md`.
