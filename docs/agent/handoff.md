# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stage 0 has landed; nothing
else of the goal has.** Stage 1 is goal 15's whole list, untouched. The design is settled in the goal
prose's standing decisions, which the pack prints in full.

Stage 0 re-pointed the fixtures written against the sentence stage 4 deletes (spec § 15's three-way
exclusivity): the case is renamed, and the two `nvs-stdlib` unit tests ground their refusals in what the
members do rather than in that sentence. **Every assertion in them survived unchanged** — under
*buffering readers share, streaming readers consume*, a streaming reader still refuses every later
reader in both directions, and the answers that actually move (`post` after `body`; `json`/`jsonAs`
joining) cannot be asserted before stage 3's `hold_body` exists.

The rest of `crates/nvs-stdlib/src/request.rs` still states the old rule — `claim_body`'s message at
`:1219`, the member comments at `:1985` and `:2067`, a test doc at `:4846`. That sweep is stage 3's,
deliberately not stage 0's, since the message is the rule and the rule is not written yet.

The thing to understand before touching anything: **a `.nvst` case answers no request today**, which is
why every request-facing member here is proven by a Rust `#[test]` rather than a case. Stage 2 is what
changes that, and the three-`.nvst`-case floor (`every_core_class_has_a_conformance_floor_of_three`)
cannot be met honestly for `json`/`jsonAs` until it lands.

## Next group

**Stage 2: the keystone — a `.nvst` case can answer a request** — one file set: `crates/nvs-test/src/`
(`lib.rs`, `case.rs`, `run.rs`) and `crates/nvs-cli/src/main.rs`. Take them in order; 2b is what makes
2a observable.

- [ ] **Stage 2a: the five sections.** `--GET--` (a query string), `--POST--` (urlencoded pairs),
      `--POST_RAW--` (the body verbatim), `--COOKIE--` and `--HEADERS--` (one field per line), spelled as
      `.phpt` spells them so the M11 corpus import stays mechanical. The table is
      `crates/nvs-test/src/lib.rs:32`, the per-section docs and the parse are
      `crates/nvs-test/src/case.rs:135` and `:154`, and `:464` is the shape of a per-line refusal.
      `--POST--` together with `--POST_RAW--` is a parse error, not a merge — the goal's standing
      decision. `rule:testing/nvst-is-separate`.
- [ ] **Stage 2b: the carrier.** A case spawns `nvs run` (`crates/nvs-cli/src/main.rs:1163`), so the
      sections cross a process boundary: `nvs run --request <file>` reads a frozen description and builds
      the `Inbound` before the program runs — `crates/nvs-cli/src/main.rs:923` is where `nvs run`'s `Ctx`
      is made, `crates/nvs-runtime/src/ctx/inbound.rs:23` is `Ctx::set_inbound`, and
      `crates/nvs-cli/src/serve.rs:323` is the worked build (`Inbound::new`, `push_header`, `set_body`).
      `crates/nvs-test/src/run.rs:332` is how `--ENV--` already reaches the spawn. A flag, never an
      environment variable, and the file format is `crates/nvs-test`'s to own — both standing decisions.
- [ ] **Stage 2c: the section that says so.** `crates/nvs-test/src/lib.rs:136`'s *What is parsed but not
      yet honoured* list records the five as honoured, beside `--ENV--` and `--ARGS--`.
      `rule:testing/nvst-is-separate`, whose "`.nvst` is unchanged" sentence is the one this goal amends.

## Backlog

- Stage 3 — the new `http-server/` fragment, `hold_body` (`crates/nvs-runtime/src/ctx/inbound.rs:602`)
  and the § 15 citations left in `request.rs` — goal prose stage 3.
- Stage 4 — `json()`/`jsonAs<T>()`, five edits each plus three `.nvst` cases each — goal prose stage 4.
- Stage 5 — `examples/json-body.nvs`, `docs/reference/core/Request.md`, a valgrind run of its own for
  the cached decoded value — goal prose stage 5.
- `docs/agent/goals/16-request-json.md:39` still names the case by the file name it had before this
  session; the goal prose is the run's input and was not edited.
- **`[context]` gap:** the pack prints `loop-goal.md`'s § *Standing decisions* but not the current
  stage's own prose section, and § *Stage 0* is where "their questions survive; the answers move" is
  defined — two `peek.py` calls to recover what the item meant.
