# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31–34 are closed**; 35 is the only one left, and its two attribute findings — D9 and D20 —
closed here.

**Both closed as correct-by-design, so item 35's remaining work is docs and one member.** D9's
answer is ADR 0071 § 2's own first bullet, "private ones included": `nvs_types::derive::codec_field`
reads a declaration's modifiers for `lateinit` alone, so encode and decode are visibility-blind by
construction, and `json-derive-encodes-declared-fields.nvst` now pins it both ways. D20's is ADR
0046 § 4: retrieval is structural so that no second namespace of attribute-kind names exists, and a
shape with no fields therefore asks for every attached literal — E0728 is the honest answer, and a
marker meant to be retrieved earns a field. Neither needed a code change.

**Two stale reference sentences fell out of that pass and are fixed**: `docs/reference/core/Json.md`
had `decodeAs<T>` refusing the top-level `array<U>` ADR 0071 § 1 spells and the binary runs, and
`docs/reference/lang/90-attributes.md`'s retrieval bullet still refused a class-constant or
enum-case payload that D21 closed. The matcher both findings turn on is
`crates/nvs-types/src/retrieval.rs:349`, not `attributes.rs` — the previous handoff's file set was
wrong about that.

**Item 35's remaining halves have no acceptance check and are open**: D6, D29, U13, M2, M3, M6, M7,
and the two `docs/reference/lang/` chapter fixes. **M3 is still a five-edit `Core` member slice, not
a documentation fix** — against `docs/spec/01-core-library.md`'s Part II class table, `Core\Test` is
missing exactly `assertContains`; the rest of that finding's roster is ADR 0079 § 24's M5-to-M8
schedule, absent on time. Item 35's own check (M10's `no_registry_card_cites_an_adr`) is green.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077,
differential 206. The driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases that
were never written; it is an open item, not a regression, and stage 0c outranks it by the goal's own
ordering.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is `E0794`;
this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed **0046 § 4** in `adrs`, and — the gap that
cost the most — `docs/reference/`'s own pages: no `[context]` field selects
`docs/reference/lang/90-attributes.md` or `docs/reference/core/*.md`, which are where every item-35
finding is actually fixed, and the pack cannot print the file the item is about. In `modules`,
`crates/nvs-types/src/retrieval.rs` and `crates/nvs-stdlib/src/attributes.rs`. Still missing, each
proven by an earlier session: `docs/spec/01-core-library.md`'s Part II class table (a `Core` class's
roster, which no field selects), **0079 §§ 4 and 24**, **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**,
**0046 §§ 2, 5**, **0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**,
**0047 § 2**, **0011**, **0086 § 6**, **0090 § 3**, **0057 § 1**, **0096 §§ 1-1a** and **0117 § 1**;
and in `modules`, `crates/nvs-types/src/enums.rs`, `lib.rs`, `intrinsics.rs`,
`crates/nvs-runtime/src/ctx.rs`, `throwable.rs`, `host.rs`, `script.rs`,
`crates/nvs-stdlib/src/task.rs`, `test.rs`, `crates/nvs-config/src/snapshot.rs`, `directive.rs`,
`crates/nvs-types/src/layout.rs`, `crates/nvs-types/src/expr/args.rs`, `attributes.rs`,
`serialize.rs`, `crates/nvs-diagnostics/src/lib.rs`, `crates/nvs-hir/src/errors.rs`,
`crates/nvs-ir/src/lower/call.rs`, `crates/nvs-cli/src/openapi.rs` and
`crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that `crates/nvs-host/src/budget.rs` matches
nothing, which is the forward anchor its own comment describes.

## Next group

**Item 35's two `docs/reference/lang/` chapter fixes, which are one file set.** Both are a divergence
the chapter does not mention; run each against `target/debug/nvs.exe` under `.agent-tmp/` before
writing the sentence, because the chapter is the third copy of a fact and the binary is the only one
that cannot be stale.

- [ ] **`20-types.md` § *Literals* gains the escapes that are not escapes and the decimal `017`** —
      `\v`, `\e` and `\f` print literally rather than as control characters, and `017` is seventeen
      rather than PHP's octal fifteen. `docs/reference/lang/20-types.md:398` is that section's
      heading, and the two facts are fixed at `crates/nvs-types/src/string_lit.rs:177` (the escape
      roster) and `crates/nvs-types/src/expr/literals.rs:594` (the leading-zero spelling); check
      whether `docs/reference/divergences.md` already owns either, and point rather than restate if
      it does.
- [ ] **`40-statements.md` names the refusal for two `catch` clauses binding one name** — the bullet
      at `docs/reference/lang/40-statements.md:303` says each `catch` variable is a declared local
      and that two clauses use two names, without saying that the second one is a compile error or
      what it is called. `docs/agent/loop-goal.md` item 35 says `E0406`, and
      `crates/nvs-diagnostics/src/lib.rs:562` is that code — `E_REDECLARED_LOCAL`, the declare-once
      rule rather than a `catch`-specific one, which is the sentence the bullet actually owes.

## Backlog

- M3: `Core\Test::assertContains` as a five-edit `Core` member — `docs/spec/01-core-library.md` Part II.
- D6: `Router::url`'s message stops naming a table that is built — `crates/nvs-stdlib/src/router.rs:539`.
- U13: a throwable's properties are writable, so "readonly" goes — `crates/nvs-types/src/error_lib.rs:133`.
- D29: `Core\Debug::render` names `await`'s result by its shape — `crates/nvs-stdlib/src/debug.rs`.
- M2, M6, M7: `Core\Fatal::onUncaughtThrow`, `Core\Command::*` and `nvs serve|fmt|convert|lsp|ctl` are
  named only as planned — `docs/reference/findings.md` § *Triage*, item 35.
- Stage 8's two unwritten isolate cases, one of which is the driver's standing acceptance failure.
