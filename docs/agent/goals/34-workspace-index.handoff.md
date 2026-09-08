# Handoff

## State

**Goal 34 — the language server grows past its first closed list — has just started; nothing of it has
landed yet.** Goal 33's whole acceptance list is this goal's floor.

The design is not open. `rule:ide/five-features-are-one-reference-index` already specifies stage 2 and 3
in full — one construction site, five readers, and the structural test that says so — and
`rule:ide/check-scope-defaults-to-open-documents` already froze `nvs.check.scope` and
`nvs.checkWorkspace`. What is *not* settled and is this goal's own record to settle: admitting
`signatureHelp`, `typeDefinition` and `implementation` against ADR 0099 § 3's test, the two completion
arms of stage 5, and reversing that same record's inlay-hint deferral in stage 6.

Two defects are live in a shipped extension right now and are stage 0's, not a later cleanup. Reproduce
both before writing anything: a cursor after `Core\` answers 37 keywords because `\` is a declared
trigger character with no arm, and `$b->` as the last statement of a block answers keywords while the
same `$b->` above another statement answers the members. The second is a hole in the `.lspt` corpus —
`crates/nvs-lsp/src/lib.rs:57`'s own example froze the case that works.

## Next group

**Stage 0, then stage 2.** They share no file set, so they are two groups; take stage 0 first because it
is small, it is user-visible, and it tells you how the completion arms are shaped before you widen the
index they will read.

- [ ] **Reproduce both stage 0 defects as `.lspt` cases** under `tests/lsp/completion/`, one per closing
      delimiter. They fail on the day they are written.
- [ ] **Fix the end-of-block recovery** in `crates/nvs-lsp/src/completion.rs`. The receiver is found by
      walking to the enclosing access node; a `}` after the cursor is what currently leaves no access
      node to find.
- [ ] **Leave `\` alone until stage 5.** Removing it from `crates/nvs-lsp/src/capabilities.rs:174` would
      have to be undone; the named test asserts every declared trigger reaches an arm, and stage 5 is
      what makes it pass. Say so in the commit rather than half-fixing it.
- [ ] **Then stage 2**, and read `rule:ide/a-full-reanalysis-stays-under-a-bound` before choosing the
      invalidation shape — the bound has to hold with the index warm, which is what rules out
      rebuilding it per keystroke.
- [ ] **The record is one, not four.** Stages 4, 5 and 6 each argue one section of it. Open it before
      stage 4, not after, because 0099 § 3's admission test is what its § *Decision* has to answer.
