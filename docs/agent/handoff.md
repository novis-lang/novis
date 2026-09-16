# Handoff

## State

**Goal `unowned-closures`, and the register is at `unowned: 15`.** `python tools/owners.py` reports
`unowned: 15`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0` and `sections outside Known gaps: 0`;
`--deferrals` is green. Every one of the 15 needs an answer only the user can give bar
`crates/nvs-runtime/src/graph.rs:74` gap 1, so the goal's own check — `unowned: 0` — is a `BLOCKED`
the moment that one is settled. 59 gaps still carry `owner: unowned-closures`.

**`rule:packaging/autoload-probes-fold-into-the-cache-key` is on disk for `autoload`.** A `UnitKey`
is `{ path, content_hash, probe_hash, env_hash }`; `crates/nvs-cli/src/script.rs`'s resolver keeps the
recorded trace in front of the unit table, spells `ProbeHash::unrecorded` for content it has not
compiled, re-keys the compile's own result, and re-asks the probed paths under the gate the content
`stat` rides — so a file written where a probe missed recompiles the unit. The rule's own paragraph
says what is left: a `discover` glob's listed directories are collected nowhere and the discovered
names hash into no key.

**Stage 3's Builds row is gone, both of its items being closed.** `requires.rs` gap 2 is struck; the
`ctor_init.rs` half closed as prose a session earlier. What stage 3 still holds is M1's two and the
**Decided** row.

**The pack did not print the rule the item named.** `[context] rules` carries the goal's four, and
stage 5 had no overlay, so `rule:packaging/autoload-probes-fold-into-the-cache-key` cost a `peek`.
`[context.stage.5]` now exists and `[context.stage.3]` names the intrinsic pass's two.

## Next group

**Stage 3: the checker's intrinsic pass** — one file set: `crates/nvs-types/src/intrinsics.rs`,
`crates/nvs-types/src/links.rs`, `crates/nvs-types/src/expr/args.rs` and
`crates/nvs-types/src/string_lit.rs`.

- [ ] **A named or spread argument reaches the pattern passes** — `crates/nvs-types/src/intrinsics.rs:86`
      (gap 4) and `crates/nvs-types/src/links.rs:46` (gap 1) are one change, which is why they are one
      slice: `check_args_typed` (`crates/nvs-types/src/expr/args.rs:48`) already builds the slot
      mapping neither pass is handed, so `Core\Str::format(template: "…")` folds and validates exactly
      as the positional spelling does. `rule:core-api/parameters-are-callable-by-name` is the rule the
      call site owes, `rule:expressions/intrinsic-list-is-closed` the one this pass lives inside.
- [ ] **A refused placeholder underlines its own offset** — `crates/nvs-types/src/intrinsics.rs:58`
      (gap 1): re-decode with positions only where a diagnostic is emitted, which is the sheet's
      answer, so the success path pays nothing; `crates/nvs-types/src/string_lit.rs` is where the
      second decoder mode belongs. `rule:expressions/intrinsic-list-is-closed`.
- [ ] **The roster grows a restriction column** — `crates/nvs-types/src/intrinsics.rs:74` (gap 3): a
      member's own restriction on a well-formed pattern — `Core\Time::parse` refusing a zonal field,
      `nvs_stdlib::cldr`'s `civil_fields_only` — is refused where it was written rather than at run
      time. `rule:expressions/intrinsic-list-is-closed`.

## Backlog

- `crates/nvs-types/src/intrinsics.rs:67` gap 2 is this goal's one ADR slot (the checker-to-IR
  prepared-pattern channel) — a session of its own, not a fourth slice.
- `crates/nvs-types/src/intrinsics.rs:93` gap 5 waits on which of two module docs is right
  (`nvs_db::sql` declines it in the other direction) — `docs/agent/carried-gaps.md`.
- The in-memory unit key's content hash is the entry file's source alone
  (`crates/nvs-cli/src/script.rs`, `observe`), so an edit to a required or autoloaded file moves
  nothing; only the artifact key hashes every reached file. Unasked against
  `rule:config/an-edit-reaches-the-next-request-without-a-restart`.
- `crates/nvs-cli/src/cache.rs:166` and `:174` share one `Decided:` — one make-executable function in
  `nvs-codegen` — and neither is testable on an x86-64 Windows host.
- `crates/nvs-runtime/src/graph.rs:74` gap 1 is the last unowned gap that does not need the user.
- A `discover` glob's listed directories join no revalidation set —
  `rule:packaging/autoload-probes-fold-into-the-cache-key`, last paragraph.
