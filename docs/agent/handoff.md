# Handoff

## State

**Goal `workspace-index`, stage 6 is landed.** `inlayHint` is `.lspt` vocabulary: `Request::InlayHint`
is in the roster, `Response::Hints` renders `L:C kind label`, `suite`'s `answer` has the arm, and
`tests/lsp/hints/` freezes the two shapes with five cases. The corpus is at 258 passing and
`every_request_answers_every_construct` is green.

**The driver's stage-1 floor check is green again.** `crates/nvs-lsp/src/hints.rs`'s known gap ended on
a paragraph rather than on its owner tag, which `owners.py` reads as an untagged gap; the paragraph now
sits above `# Known gaps`, where the playbook bullet on that message says to put it.

**What the goal still owes is its one ADR**, and nothing else. `docs/agent/loop-goal.md` § *Standing
decisions* names what it covers; the number 0171 was free at this commit and is worth re-deriving.

**One fact the record's stage-4 paragraph needs and no file states plainly:** `Request::ALL`
(`crates/nvs-lsp/src/case.rs:92`) carries M10's four *case* additions — `codeLens`, `references`,
`documentHighlight`, `inlayHint` — while `signatureHelp`, `typeDefinition`, `implementation` and
`typeHierarchy` are answered on the wire only (`crates/nvs-lsp/src/server.rs:49`, `:278`) and have no
`.lspt` vocabulary. The rule amendment has to say which list it is naming.

## Next group

**Stage 6: the goal's one ADR** — one file set: `docs/decisions/0171.md` (new), `docs/rules/ide.json`,
`docs/rules/ide/the-request-set-is-closed.md`.

- [ ] **The record and the rule it amends, in one commit.** Write `docs/decisions/0171.md` to
      `docs/agent/conventions.md` § *A decision record*'s shape — re-derive the number from
      `docs/decisions/` first — covering stage 2's index shape, stage 4's request admissions against
      [ADR 0099](../decisions/0099.md) § 3's test, stage 5's namespace and bare-name arms, and stage
      6's reversal of that record's inlay-hint deferral. Its `changes.modifies` names
      `ide/the-request-set-is-closed`; the entry at `docs/rules/ide.json:181` gains `"0171"` in
      `because`, and the fragment `docs/rules/ide/the-request-set-is-closed.md:1` gains M10's
      additions beside M4B's nine — its closing paragraph currently says inlay hints "stay" at M10,
      which is the sentence this goal made false. Then `python tools/rules.py --render`, which
      `session.py --wrap` runs as a gate anyway.

## Backlog

- `crates/nvs-lsp/src/hints.rs:58`'s known gap is `unowned`: widening past `ExprInfo::Call` is a
  table question, and no goal holds it — `docs/agent/carried-gaps.md` if this goal retires first.
- The `Int` construct is reached by `inlayHint` in the Rust test and by no `.lspt` case, per the
  playbook bullet this session added.
