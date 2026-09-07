# Handoff

## State

**Goal 14 stage 4's document store is landed and whole.** `crates/nvs-lsp/src/document.rs` holds a
buffer per URI with its version, overlays every open buffer on the `SourceMap` so the whole
`require` graph reads unsaved text, keeps the reverse index an edit is republished from, and drops
an analysis started for a version the client has already replaced.
`crates/nvs-lsp/src/position.rs` is the wire's positions and holds no arithmetic — the conversions
are `nvs-diagnostics`' (`rule:ide/positions-have-one-home`).

`nvs-lsp` now links `nvs-diagnostics`, `nvs-syntax` and `nvs-hir`, and `no_crate_the_server_links_writes_to_stdout` still passes over the widened closure.

**No request is answered.** `crates/nvs-lsp/src/server.rs:67` applies `didOpen`/`didChange`/
`didClose` to the store and publishes nothing; `crates/nvs-lsp/src/suite.rs:157`'s `answer` is still
the seam every request slice lands one arm in, and `tests/lsp/` is still empty, so the goal's
`lsp cases` check (160 passing) stays red until the request slices land.

**The analysis thread `rule:ide/the-server-is-synchronous` names is deliberately not spawned yet**,
and `crates/nvs-lsp/src/document.rs`'s module doc is where that decision is written: what the rule
needs at this stage is that a superseded answer is dropped, which is a fact about versions rather
than about threads. It arrives with the first answer that has somewhere to go.

## Next group

**Stage 5: diagnostics, phase-gated and published** — one file set: `crates/nvs-lsp/src/document.rs`,
a new `crates/nvs-lsp/src/diagnostics.rs`, `crates/nvs-lsp/src/server.rs`, with
`crates/nvs-lsp/src/render.rs` and `crates/nvs-lsp/src/position.rs` read only.

- [ ] **The type phase joins the analysis, and the gate over it.** A file that produced an `E00xx`
      or `E01xx` diagnostic publishes those and its declaration diagnostics and suppresses its own
      `E03xx`/`E04xx`, for that file alone. `rule:ide/diagnostics-are-phase-gated`; extend
      `crates/nvs-lsp/src/document.rs:296`'s `analyse`, which stops at `resolve_program` today, the
      way `nvs-cli`'s `front_end_granted` continues into `nvs_types::check_program`. Tests
      `a_parse_error_suppresses_the_type_diagnostics_of_that_file_only` and
      `a_resolution_error_does_not_suppress_a_type_error`.
- [ ] **A diagnostic crosses to the wire.** `Code` becomes `code`, the span becomes a `Range`
      through `crates/nvs-lsp/src/position.rs:50`'s `position_at`, and `codeDescription` is left
      unset because there is no documentation site to point one at (ADR 0099 § 3's last paragraph).
      The rendering `crates/nvs-lsp/src/render.rs:109` freezes is what a case compares against.
      Test `a_diagnostic_carries_its_code_and_no_code_description`.
- [ ] **Publishing, for open documents only.** `crates/nvs-lsp/src/server.rs:67` publishes after an
      edit for `crates/nvs-lsp/src/document.rs:232`'s `to_republish`, checking
      `crates/nvs-lsp/src/document.rs:218`'s `is_current` again before it sends, and `phase=all`
      defeats the gate for a case. `rule:ide/an-open-document-is-its-own-entry-point`. Test
      `phase_all_publishes_what_the_gate_suppressed`, plus the first `.lspt` cases under `tests/lsp/`.

## Backlog

- 160 passing `.lspt` cases is stage 4's other check; every request slice ships its own.
- A BOM is a column of line 0 in the source map, and VS Code's buffer excludes it — so a `Location`
  answered for a *closed* BOM file is one column out on line 0 only. The definition slice decides it;
  `crates/nvs-diagnostics/src/source.rs`'s `a_position_round_trips_through_utf16_and_utf8` is where
  the current answer is pinned.
- `nvs.lsp.debounce`'s 150 ms is unimplemented, and belongs with the analysis thread.
- `Documents::iter`/`len`/`is_empty` exist for the publish loop and have no caller until stage 5.
- `nvs lsp-test --coverage` and `every_request_answers_every_construct` are unwritten
  (`rule:ide/lspt-coverage-is-inferred`), and are stage 9's gate.
