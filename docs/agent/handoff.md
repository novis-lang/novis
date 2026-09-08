# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 5 has landed on the document side; the
binding site is what the goal still owes.** The tree is green (`verify.py`, 8 of 8).

**The generated API document answers a capture at a `Parses` class with a bare `{"type": "string"}`.**
`crates/nvs-cli/src/openapi.rs:390`'s arm is it, and it sits *below* the `Core\Uuid` name arm on
purpose: `Core\Uuid` implements the same contract, so a rule reading the contract alone would drop
`format: uuid` from the one type the engine ships. The `schema` function's doc comment is the home of
why a class may not name its own format.

**The row is what tells a class from an enum, because the rendering cannot.**
`crates/nvs-types/src/routes.rs`'s `RouteParam::parses` is a sixth field, set at both builders — the
capture walk and the `#[Query]` walk — from `crate::commands::is_parses_class`, which is
`converts_from_string`'s class arm asked on its own. An enum keeps the empty schema, because its case
spellings are `Core\Router::match`'s to decide and are not decided yet; a union of literal types keeps
its bare `enum` with no `type` beside it, which `openapi.rs`'s existing doc owns.

**What the goal still owes**, all of it in the next group: the binding site
(`crates/nvs-runtime/src/routes.rs:76`, gap 3), the class-typed exception in
`rule:security/route-capture-is-laundered-by-its-type` and `rule:routing/a-bad-query-value-is-a-400`,
ADR 0160 — the rules and the record being one commit, and that commit being the one that writes the
binding site — and `examples/parses.nvs`, which is stage 5's `exact` check and the acceptance failure
the driver has been reporting. `docs/novis.md:14048`'s "a path whose capture will not convert is
claimed by nobody" goes stale the same day.

## Next group

**Stage 5: the binding site, and the fixture that proves it** — one file set:
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-stdlib/src/router.rs`, `examples/parses.nvs`.

- [ ] **A capture typed at a `Parses` class converts at the binding site, and a segment the class
      refuses is a `400`** — `crates/nvs-runtime/src/routes.rs:76` is gap 3, which states what is
      missing and what is settled; `crates/nvs-runtime/src/commands.rs:122` is `ArgConv::Parses`, the
      one home of what reaching a compiled `parse` costs and the route this must copy; and
      `crates/nvs-stdlib/src/router.rs:1015` is where a `Param` crosses to the program today. The rule
      edits to `rule:security/route-capture-is-laundered-by-its-type` and
      `rule:routing/a-bad-query-value-is-a-400`, and ADR 0160 behind them, are **this same commit** —
      the goal's § *Standing decisions* says so, and `conventions.md` § *A decision record* is why.
- [ ] **`examples/parses.nvs`, stage 5's `exact` check** — five `want` lines under `nvs run --request`,
      the last two pinning that a class refuses in a different place than the router does.
      `crates/nvs-cli/tests/fixtures/api/parses.nvs` is a compiling `implements Parses` class to copy
      the shape from; `crates/nvs-runtime/src/routes.rs:424` is the native `Core\Uuid` arm whose
      refusal is the `404` half.

## Backlog

- ADR 0160 is the one number this goal may open — `docs/decisions/`, next free re-checked at the file.
- `docs/novis.md:14048`'s `Core\Router::methodsFor` sentence, stale the day the binding site lands.
- The `[context] rules` manifest in `docs/agent/loop-goal.toml` is missing
  `attributes/api-adds-and-cannot-contradict`, which stage 5's own item names as what the document
  owes; add it before the next document slice.
- `crates/nvs-cli/tests/fixtures/api/` is not enumerated by anything, so a new fixture there costs no
  registration — `docs/agent/handoff.md` is the wrong home for that if it recurs.
