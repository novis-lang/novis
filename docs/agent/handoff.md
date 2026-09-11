# Handoff

## State

**Goal `workspace-index` is landed whole, including its one ADR.** [0171](../decisions/0171.md)
covers stage 2's index shape, stage 4's request admissions against [0099](../decisions/0099.md) § 3's
test, stage 5's completion arms and stage 6's reversal of that record's inlay-hint deferral; its
`changes.modifies` names `ide/the-request-set-is-closed`, whose fragment now carries M10's eight
requests beside M4B's nine and says which four of the eight have `.lspt` vocabulary.

**The stage-1 floor check was red at this session's start, not green.** The previous repair moved
`crates/nvs-lsp/src/hints.rs`'s gap tag onto the item's last line, which made it readable as
`unowned` — and an `unowned` tag owes a reason bullet naming that path. `docs/agent/carried-gaps.md`
§ *Unowned* now carries it, and `python tools/owners.py --check --reasons` resolves every tagged gap.
With that closed, `python tools/loop.py --goal-only` prints *GOAL REACHED: every acceptance check
passes*, so this session's status line claims the goal rather than a slice.

**Two things this session left standing on purpose.** `ide/the-request-set-is-closed` is still
`status: designed` though the server answers every request it names (`docs/rules/ide.json:183`) —
moving it to `shipped` publishes it into `docs/novis.md` and is a claim about the whole M4B client,
not about this crate. And `docs/plan/m4b.md:52` and `:155` still say "nine standard requests",
which is M4B's own history and correct where it stands.

## Next group

**No stage: the goal's own stages are closed** — one file set:
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/case.rs`, `crates/nvs-lsp/tests/coverage.rs`.

- [ ] **A rendering for the four wire-only requests, so each becomes a `.lspt` case.**
      `signatureHelp`, `typeDefinition`, `implementation` and `typeHierarchy` are answered at
      `crates/nvs-lsp/src/server.rs:278`, `:260`, `:266` and `:337` but carry no canonical spelling,
      which is why `crates/nvs-lsp/src/case.rs:98`'s `Request::ALL` does not name them
      (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`: a case may not invent one). Add each
      shape to `crates/nvs-lsp/src/render.rs:1`'s renderer first — [0171](../decisions/0171.md) § 2
      is the reasoning, and it says giving one a rendering changes nothing about the answer.
- [ ] **Then the roster row and the cases.** Each request added to
      `crates/nvs-lsp/src/case.rs:98` becomes a row `crates/nvs-lsp/tests/coverage.rs:57`'s
      `every_request_answers_every_construct` requires at every construct, so the cases under
      `tests/lsp/` land in the same slice as the roster entry or the gate goes red.

## Backlog

- `crates/nvs-lsp/src/hints.rs` gap 1 — parameter hints only for `ExprInfo::Call`; unowned with its
  reason in `docs/agent/carried-gaps.md` § *Unowned*.
- `ide/the-request-set-is-closed` and its neighbours are `status: designed` with shipping code
  behind them — `docs/rules/ide.json`, a sweep and not a slice.
- `python tools/decisions.py --work` is the user-fired summary chore — `docs/agent/decisions-summary.md`.
