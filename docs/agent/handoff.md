# Handoff

## State

**Goal `unowned-closures`, stage 2.** Gap 6's compiled-code half is closed: **both** raises compiled
code makes now carry the site they happened at, and only a helper's raise still carries none.

`nvs_raise_new` (`crates/nvs-runtime/src/throwable.rs:987`) takes a fifth operand — the raising
statement's carrier — and decodes it through `Thrown::capture_site` exactly as `nvs_raise` does, so a
`catch` beside a checked operator reads `Ratio::of() at file:line` and a matching `location` instead
of an empty trace. The channel is `Inst::raise_site` (`crates/nvs-ir/src/ir.rs:450`), set only by
`Lowering::emit_raising` (`crates/nvs-ir/src/lower/mod.rs:2201`) on the checked arithmetic rows, off
the same `Lowering::source` a `throw` in that statement takes. It carries the **datum**, not a
`SourceConst` operand: `nvs-codegen` bakes the blob in the cold block it already raises from
(`crates/nvs-codegen/src/emit.rs:2076`), so the path that does not overflow spends no instruction on
it and `rule:errors/throw-is-not-slower` — amended, rendered, mirrored to `website/src` — still holds.
A site-less raise takes `Ctx::raise` unchanged.

Nothing is blocked. `python tools/rules.py --check` is red tree-wide on a goal file that cites the
rule its own acceptance check exists to create (`docs/agent/goals/61-class-scoped-types.toml:237`
passes a `types/class-scoped-alias` citation to `peek.py` as the check's own argument), so
`session.py --wrap` refuses every wrap while that goal is queued; this session's commits were made by
hand for that reason, as the last one's were. Stage 1's floor is goal `m8-stdlib-depth`'s whole list,
carried and untouched.

## Next group

**Stage 2: the helper half of gap 6 — one design call, then the case that pins it** — one file set:
`crates/nvs-runtime/src/ctx/error.rs`, `crates/nvs-runtime/src/throwable.rs`,
`crates/nvs-ir/src/lower/mod.rs` and `crates/nvs-codegen/src/emit.rs`.

- [ ] **A helper's raise is given a site, or the bound is written** — `Fault` and
      `crates/nvs-runtime/src/ctx/error.rs:394`'s `raise_with_slots` build an exception with none,
      and the carrier a producer *is* handed (`crates/nvs-runtime/src/source.rs:138`'s `of_operand`)
      reaches only `nvs_stdlib::registry`'s `SOURCE_MEMBERS` rows. Two builds, and the cheaper one is
      **not** the ABI change: (a) a site operand on the helper ABI (`rule:errors/helper-abi`) is every
      helper's signature, every `HelperCall` lowering, and a blob per fallible call site in the data
      section; (b) seeding it in the landing block that *catches in frame* — the one place no label is
      ever pushed, `crates/nvs-ir/src/ir.rs:2740`'s `Terminator::Catch` — is one new primitive called
      on an error path that already exists, nothing per propagating frame, and reuses
      `Inst::raise_site` (`crates/nvs-ir/src/ir.rs:450`) as the channel. Weigh them against
      `rule:errors/throw-is-not-slower`, build one, and strike gap 6 at
      `crates/nvs-runtime/src/lib.rs:216` either way — the goal's § *Standing decisions* admits a
      bound but not a silence.
- [ ] **The rule's own conformance case reads the arithmetic raise too** —
      `tests/conformance/core/a-record-names-the-member-it-was-produced-in.nvst:23` already pins
      `$e->location` for a `throw`; the checked operator is the second producer of that one datum
      (`rule:errors/a-record-names-where-it-was-produced`), and the Rust half is landed at
      `crates/nvs-codegen/tests/throwing.rs:216`.

## Backlog

- `crates/nvs-runtime/src/lib.rs:199` gap 2 — `nvs_str_concat`/`concat_n` reuse a solely-owned left
  operand, with the ownership hand-off in `nvs-ir`'s lowering for those two calls.
- `crates/nvs-runtime/src/lib.rs:209` gap 5 and `:234` gap 7 — one decision, "a collector that runs
  only near the memory ceiling", so they are one build and not two.
- Stage 3's failing acceptance check (`nvs-hir`'s require-path decode and the autoload probe's unit
  key) is an artefact nothing has written yet, not a regression.
- `docs/agent/carried-gaps.md`'s `nvs-runtime` bullet now covers the string and backtrace halves
  only; its `[until:]` trailer still holds while gaps 2, 6 and 7 name this goal.
- The website mirror is `node scripts/sync-rules.mjs`, run from `website/`, and a rule edit needs it:
  the page holds a verbatim copy of the fragment.
