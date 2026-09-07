# Handoff

## State

**Goal 14 stage 5 is two thirds landed.** `crates/nvs-lsp/src/document.rs`'s `analyse` continues
into `nvs_types::check_program` the way `nvs-cli`'s `front_end_granted` does — with no grants, since
the editor names no `--config` — so `E03xx` and `E04xx` exist to be published at all.
`crates/nvs-lsp/src/diagnostics.rs` is the presentation half: `phase_gated` is
`rule:ide/diagnostics-are-phase-gated` as a filter over a finished walk, and `to_wire` is one
`nvs_diagnostics::Diagnostic` as one `lsp_types::Diagnostic`.

**`nvs-lsp` now links `nvs-types`**, which puts `nvs-stdlib` and `nvs-runtime` under the server.
`tests/stdout_policy.rs` exempts the runtime's own `stdout()` — a sink a caller wires into a `Ctx`,
and the server builds none — and a second test pins that nothing between them wires one.
`rule:ide/stdout-belongs-to-the-protocol` carries the reasoning.

**Nothing is published yet.** `crates/nvs-lsp/src/server.rs:116`'s `apply` still applies
`didOpen`/`didChange`/`didClose` to the store and sends nothing back,
`crates/nvs-lsp/src/suite.rs:157`'s `answer` is still the seam every request slice lands one arm in,
and `tests/lsp/` is empty, so the goal's `lsp cases` check stays red. Three of the stage's four
acceptance tests exist and pass; `phase_all_publishes_what_the_gate_suppressed` is the open one.

## Next group

**Stage 5: publishing, which closes the stage** — one file set: `crates/nvs-lsp/src/server.rs`,
`crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/document.rs`, with
`crates/nvs-lsp/src/diagnostics.rs` and `crates/nvs-lsp/src/render.rs` read only.

- [ ] **The server publishes, for open documents only.** After a notification,
      `crates/nvs-lsp/src/server.rs:116`'s `apply` analyses and sends
      `textDocument/publishDiagnostics` for every URI `crates/nvs-lsp/src/document.rs:232`'s
      `to_republish` names, asking `crates/nvs-lsp/src/document.rs:218`'s `is_current` again
      immediately before each send, and publishing an empty list when a document's diagnostics are
      gone so the client clears them. One document's own file only — a diagnostic in a `require`d
      file that nobody opened is workspace scope, which is M10's.
      `rule:ide/an-open-document-is-its-own-entry-point`.
- [ ] **`phase=all` defeats the gate for a case.** `crates/nvs-lsp/src/suite.rs:157`'s `answer`
      grows its `diagnostics` arm: `phase_gated` by default, `crate::Analysed::diags` whole when the
      request line says `phase=all`, both rendered by `crates/nvs-lsp/src/render.rs:154`'s
      `Response::Diagnostics`. `crates/nvs-lsp/src/case.rs` already closes the argument set
      (`rule:ide/a-request-line-is-closed`), so check `phase` is in it before adding one. Test
      `phase_all_publishes_what_the_gate_suppressed`.
- [ ] **The first `.lspt` cases, under `tests/lsp/`.** The gate in both directions as M4B's
      acceptance paragraph words it — `E0102` and not `E0301` gated, both with `phase=all` — plus a
      document whose unclosed brace does not stop diagnostics on the well-formed code around it.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`; the shape is `conventions.md` § *An `.lspt`
      case*, and `crates/nvs-lsp/src/suite.rs:68`'s `collect` is what walks the directory they go in
      — no registration, so the only thing to get right is the path and the sections.

## Backlog

- `analyse` drops the `TypeInterner` and `ExprTypeTable` the type phase filled; hover and definition
  will want them kept rather than recomputed — `crates/nvs-lsp/src/document.rs`'s `analyse`.
- A secondary label is not carried as `relatedInformation`, a `Suggestion` is not a code action, and
  no `Unnecessary` tag is set — all three named in `crates/nvs-lsp/src/diagnostics.rs`'s `to_wire`.
- The goal's `lsp cases` check wants 160 passing and `tests/lsp/` holds none.
- The analysis thread `rule:ide/the-server-is-synchronous` names is still not spawned;
  `crates/nvs-lsp/src/document.rs`'s module doc owns why and when.
