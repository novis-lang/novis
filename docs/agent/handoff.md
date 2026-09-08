# Handoff

## State

**Goal 19 — stage 4's `cargo-named` check is green, in the two crates it is actually split across.**
Nothing about the arm needed writing: `crates/nvs-runtime/src/routes.rs:450` already crossed a
`Parses` capture as its class and text, `crates/nvs-stdlib/src/router.rs:1065` already converted one
at the binding site, and `crates/nvs-stdlib/src/command.rs:539` already converted a `decimal`
argument. What was missing was the tests, and **two of the five names could not live in
`nvs-runtime` at all** — `capture_value` and `parse_each` are one crate above the tables, because
only a binding site holds the armed class table a compiled `parse` is reached through. The check is
now two blocks: `-p nvs-runtime` for the match, `-p nvs-stdlib` for the binding site, with the stage
comment saying why the split is the decision rather than a filing accident.

**Stage 4's other check and every stage 5 artefact are already on disk** — the three `.nvst` cases
under `tests/conformance/core/`, `examples/parses.nvs` with its `.nvsr` request, and both OpenAPI
tests at `crates/nvs-cli/tests/openapi.rs:518`. `python tools/verify.py` is 8 of 8 green. So the
next acceptance run is what says whether anything is left, and the group below is written to be
checked rather than built.

**Still open and not this stage's:** `tryParse` is unreachable on a user implementor, nothing binds
a `#[Query]` parameter, and a class-typed `#[Query]` cannot carry a default (`E0451` wants a literal
of the declared type) — which is why stage 5's fixture reads its two `tag` lines by hand.

## Next group

**Stage 5: the proofs — one file set: `docs/agent/loop-goal.toml`, `examples/parses.nvsr`,
`crates/nvs-cli/tests/openapi.rs`.** Every artefact exists, so each item below is a run-and-read
first and a repair only if it is red.

- [ ] **The program leg, run rather than assumed** — `docs/agent/loop-goal.toml:5951` is the
      `stage = "5 the proofs"` fixture, and its `want` list is five lines the leg must print.
      `target/debug/nvs.exe run --request examples/parses.nvsr` is the whole check; the comment
      above it says `nvs run` installs the route table and matches nothing against it, so the
      fixture takes the door's walk itself. `rule:security/route-capture-is-laundered-by-its-type`
      is what its last two lines pin — a class's refusal is a `400`, a `Core\Uuid` the router
      refuses is no match.
- [ ] **The OpenAPI pair** — `crates/nvs-cli/tests/openapi.rs:518`'s
      `a_parses_capture_is_a_string_schema` and `core_uuid_keeps_its_named_format_over_that_schema`,
      against the arm at `crates/nvs-cli/src/openapi.rs:386`. The goal's § *Standing decisions*
      settles the shape and is not to be re-derived: a `Parses` class answers `{"type": "string"}`
      and `Core\Uuid` keeps its `format: uuid`, because a named format is a documentation hint over
      a conversion rather than a conversion rule.
- [ ] **The stage's four remaining blocks** — `docs/agent/loop-goal.toml:5972`, `:5982` and `:5995`
      are the `nvs-stdlib` gates, the reference regeneration and the rest of stage 5. They are
      whole-suite gates rather than new work, so read the driver's report before opening anything.

## Backlog

- `#[Query]` binds nothing yet; a class-typed one cannot carry a default (`E0451`) — `rule:routing/a-bad-query-value-is-a-400` says the query half is not shipped.
- `tryParse` is unreachable on a user implementor — `rule:expressions/try-parse`, and `nvs_types::layout` enters no method for an interface default.
- An enum *subset* is still `ArgConv::Unconverted` — `crates/nvs-runtime/src/commands.rs`'s known gap 1, and not this goal's.
- A capture at an enum still hands its segment text over — `rule:routing/a-capture-narrows-to-a-closed-set`'s last paragraph.
- This clone has no `core.hooksPath` set; `verify.py` says so every run — `docs/setup.md`.
