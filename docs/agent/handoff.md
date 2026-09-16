# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 15`.** `python tools/owners.py` reports
`unowned: 15`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0` and `sections outside Known gaps: 0`;
`--deferrals` is green. Every one of the 15 needs an answer only the user can give bar
`crates/nvs-runtime/src/graph.rs:74` gap 1, so the goal's own check — `unowned: 0` — is a `BLOCKED`
the moment that one is settled. 54 gaps still carry `owner: unowned-closures`.

**The three roster passes read the call's argument mapping.**
`crate::expr::args::argument_filling` is `check_args_typed`'s `ArgSlot` mapping read back, and
`crates/nvs-types/src/intrinsics.rs`, `links.rs` and `reasons.rs` address their argument through it
— so `Core\Str::format(template: …)`, `Core\Router::url(name: …)` and `Core\Html::toSource(reason:
…)` are read exactly as the positional spellings are. A `...` still says nothing about a variadic
*tail* (its count is a run-time fact), and that is stated where it is decided rather than carried as
a gap. Four module-doc gaps struck with it, and the carried-gaps entry that indexed them.

**The intrinsic roster has a restriction column.** `Intrinsic::restriction` carries a member's own
rule about a pattern its grammar reads — today `Restriction::CivilFields`, which is
`nvs_stdlib::cldr::validate_civil`, so a literal zonal pattern at `Core\Time::parse` is refused
while compiling. Two `.nvst` cases moved that pattern into a variable to keep the runtime throw
they pin reachable.

**Both slices are one commit, because they share `intrinsics.rs`.** Splitting them would have left
the first commit calling a `pub fn` the second adds, so the history takes one commit naming both.

**Stage 3's file set is down to `intrinsics.rs` gaps 1 and 2.** Gap 1 (the diagnostic's own offset)
wants a decoder that records positions, which `crates/nvs-types/src/string_lit.rs` does not hold —
that path is `[context] modules`' one dead pattern and the module does not exist under that name.
Gap 2 is this goal's ADR slot.

## Next group

**Stage 3: the checker's intrinsic pass, what is left of it** — one file set:
`crates/nvs-types/src/intrinsics.rs`, whatever holds the string-literal decoder, and
`crates/nvs-types/tests/intrinsics.rs`.

- [ ] **Find where a string literal is decoded, and say so in the manifest** —
      `crates/nvs-types/src/intrinsics.rs:75` (gap 1) names `crate::string_lit`, which no file in
      the tree matches: `orient.py` prints that `[context] modules` pattern as dead every session.
      Locate the decoder (`python tools/peek.py --locate` over `crates/nvs-types/src`), then fix
      both the gap's own reference and the manifest pattern in `docs/agent/loop-goal.toml`.
- [ ] **A refused placeholder underlines its own offset** — `crates/nvs-types/src/intrinsics.rs:75`
      (gap 1): the decided option is a second decoder mode that records positions, reached only when
      a diagnostic is emitted, so the success path stays what it is. `report_malformed`
      (`crates/nvs-types/src/intrinsics.rs:775`) is the one place every grammar's refusal passes
      through, and `rule:expressions/intrinsic-list-is-closed` is the pass this lives inside.
- [ ] **Nothing is prepared yet** — `crates/nvs-types/src/intrinsics.rs:84` (gap 2) is this goal's
      one ADR slot: the checker-to-IR channel that carries a prepared pattern into the artifact
      cache (`crates/nvs-cli/src/cache.rs:2491` names it from the other end). Take it only with the
      two slices above landed, and open the next free record for it.

## Backlog

- `crates/nvs-types/src/intrinsics.rs:91` gap 3 waits on which of two module docs is right about
  `rule:core-classes/db-literal-query-checking`'s unterminated string literal (`nvs_db::sql`
  declines it in the other direction).
- The 15 `unowned` items are a `BLOCKED` for the user bar `crates/nvs-runtime/src/graph.rs:74`
  gap 1 — `python tools/owners.py` lists them.
- `docs/agent/carried-gaps.md` still holds 61 items; the ones this goal closes go out with the
  module-doc gaps they index.
