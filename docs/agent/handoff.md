# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/lib.rs`'s gap 1 is
**closed and its item deleted**: spec § 13 now has a walk and a ratchet of its own, so nothing past
§ 12 is unmeasured except the two sections whose bound is stated. `python tools/owners.py --closes
decided-closures` names 14, down from 15; `crates/nvs-stdlib/src/test.rs:94` is next.

**A § 13 roster is the run of code spans a signature or a generic anchors.** § 13 writes
`| Class | Owns | ADR |`, one row per class, with the roster and a paragraph about it sharing the
*Owns* cell. `compiler_facing_members` accepts a span that writes `name(` or `name<`, and any span
commas and slashes alone join to one that does; every other gap — a full stop, an em dash, a word —
ends the run, which is what keeps a *Replaces* clause's PHP twins, `Core\Command`'s `string` and
`Core\Decimal`'s `decimal` out. `the_compiler_facing_walk_reads_a_roster_and_not_the_prose_beside_it`
pins both directions. §§ 16-17 stay out: their cells are essays with no run to find an end to, and
their `Class` column keeps the class-level walk it already had.

**Two keys are `unowned`, and that is the honest column rather than a deferral.**
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt` holds `Core\BigInt` and the
five `Core\Test` members § 13 names — `double`, `partial`, `assertCalled`, `assertNeverCalled`,
`assertCompletes`. No live goal builds either and no milestone's plan carries them: M8 scoped
`Core\BigInt` beside `Core\Decimal` and closed with the decimal half, so a milestone tag would name a
row already green. Both are indexed in `docs/agent/carried-gaps.md` § *Unowned* with what has to be
decided; they are a scheduling question for the user, and goal `gap-zero` will meet them.

## Next group

**Stage 4: `crates/nvs-stdlib/src/test.rs`'s two gaps** — one file set, the assertion surface and the
`Ctx` behind it: `crates/nvs-stdlib/src/test.rs` and `crates/nvs-runtime/src/ctx/error.rs`.

- [ ] **`crates/nvs-stdlib/src/test.rs:94` — gap 1, struck as a stated bound.** The `Decided:`
      sentence is *No: keep the runtime throw naming `assertEqualsDeep`*, so the work is prose, not
      code: write what the module does now — a non-`Comparable` object under `assertEquals` is a
      catchable throw here rather than the compile error `rule:testing/assertions-are-typed` names,
      because the refusal wants the class graph `nvs_types` holds and this crate does not — then
      delete the numbered item and its `— owner:` line. Amend the rule's fragment in the same slice
      if it promised the compile error unconditionally.
- [ ] **`crates/nvs-stdlib/src/test.rs:102` — gap 2, built.** `Decided: state it in the embedding
      contract and assert it when a `Ctx` is built`. `assertThrows` matches a class by name through
      `crates/nvs-runtime/src/ctx/error.rs:583`'s `pending_conforms_to`, which reads ancestry off a
      descriptor no helper-raised failure carries until
      `crates/nvs-runtime/src/ctx/error.rs:247`'s `set_runtime_error_class` has installed one. Assert
      it where a `Ctx` is built, state the requirement in the embedding contract, then delete the
      item and its `— owner:` line.

## Backlog

- `crates/nvs-stdlib/src/ast.rs:83`, `cli.rs:132`, `command.rs:74` — the next stage-4 items after
  test.rs, each a `Decided:` sentence in its own module doc.
- `crates/nvs-stdlib/src/db/mod.rs:231` and `:257` — two gaps in one file set, with `regex.rs:69` and
  `response.rs:197` after them.
- `crates/nvs-syntax/src/lib.rs:96`, `crates/nvs-types/src/defaults.rs:58`,
  `crates/nvs-types/src/response.rs:29` — the three outside `nvs-stdlib`.
- `crates/nvs-diagnostics/src/embedded.rs:30`, `crates/nvs-stdlib/src/html.rs:73` — the remaining two.
- `Core\BigInt` and `Core\Test`'s double half are `unowned` ratchet keys now; the user's call,
  `docs/agent/carried-gaps.md` § *Unowned*.
