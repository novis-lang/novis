# Handoff

## State

**Goal 14 stage 2 is landed and whole.** `crates/nvs-lsp` exists on `lsp-server` 0.10 and
`lsp-types` 0.97, synchronous over stdio, and `nvs lsp` starts it. `initialize` declares
`rule:ide/the-request-set-is-closed`'s whole list plus `serverInfo`, the position encoding is
negotiated per LSP 3.17, and `tests/stdout_policy.rs` holds `rule:ide/stdout-belongs-to-the-protocol`
over the closure rather than over one crate. `cargo deny check` is green and
`THIRD-PARTY-LICENSES.txt` is regenerated for the six crates that entered the distributed graph.

**No request is answered.** Everything but `shutdown` gets `MethodNotFound` from
`crates/nvs-lsp/src/server.rs:60`, on purpose: the case format lands before any handler, because a
request built before `.lspt` exists is one nobody can prove.

**Stage 3 has nothing on disk** — no `.lspt`, no `tests/lsp/`, no `nvs lsp-test`, no
`nvs_lsp::render`. Position conversion is still `nvs-diagnostics`' alone; none is written here and
none may be.

Capabilities are declared for the whole goal rather than stage by stage — a client caches them at
`initialize` and never re-reads, so switching one on later means an editor that started against an
earlier binary keeps asking nothing. The reasoning is `crates/nvs-lsp/src/capabilities.rs`'s module
doc; a stage that adds a handler adds no capability.

## Next group

**Stage 3: the `.lspt` case format, its runner, and the gate** — one file set: two new modules under
`crates/nvs-lsp/src/`, `crates/nvs-test/src/section.rs` read only, `crates/nvs-cli/src/main.rs`, and
a new `tests/lsp/`.

- [ ] **The case reader.** A new `crates/nvs-lsp/src/case.rs` over `nvs_test::section::lex`
      (`crates/nvs-test/src/section.rs:48`, `Section` at `:17`, `header` at `:74`) — one section
      lexer for both formats and nothing else shared. `--FILE <path>--` carries its own path.
      `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`; the format's home is
      `crates/nvs-lsp/src/lib.rs:1`'s module doc. Tests
      `an_lspt_case_reads_its_sections_through_the_shared_lexer` and
      `a_second_file_section_carries_its_own_path`.
- [ ] **The cursor and the closed request line.** Exactly one `<|>`, removed before analysis and
      reported as an offset, and none where the request takes none; `--REQUEST--` is one line whose
      `key=value` set is closed per request, so an unknown argument fails the case loudly — the
      header parser it reuses is `crates/nvs-test/src/section.rs:74`.
      `rule:ide/a-request-line-is-closed`, argument set in ADR 0099 § 5. Test
      `exactly_one_cursor_is_required_where_the_request_takes_one`.
- [ ] **One home for a rendering.** A new `crates/nvs-lsp/src/render.rs`, joining the export block
      at `crates/nvs-lsp/src/lib.rs:38` — diagnostics as
      `L:C-L:C severity CODE message`, hover verbatim, definition as `file:L:C` or `none`,
      completion as `label kind detail`, semantic tokens as `L:C+len type modifiers`, symbols as an
      indented outline. `rule:ide/the-rendering-has-one-home`. Test
      `every_response_kind_renders_through_one_module`.
- [ ] **`nvs lsp-test <paths>`**, walking directories for `*.lspt` and printing
      `N passed, M failed` — the line `tools/loop.py`'s `nvs-suite` check kind already parses.
      Beside the `Lsp` variant at `crates/nvs-cli/src/main.rs:413` and its arm at
      `crates/nvs-cli/src/main.rs:853`. `.lspt` and `.nvst` stay two suites and share no summary.

## Backlog

- `nvs lsp-test --coverage` and `every_request_answers_every_construct` — the gate that decides when
  there are enough cases; `rule:ide/lspt-coverage-is-inferred`, goal file stage 3.
- The `min_passing = 160` `.lspt` floor is a count, not the gate; goal 14's stage 4-9 checks.
- `docs/reference/tools/40-editor.md` does not exist — goal 14 stage 11 creates it with
  `# nvs lsp` and `# nvs lsp-test`, each owing `rule:testing/four-proofs`' set.
- `Diagnostic::suggestions` is carried by three producers and read by nothing; filling it for the
  casing and legacy-cast codes is stage 9's actual work, not the code-action plumbing.
- The `[context] adrs` manifest did not print ADR 0099 § 4, and the semantic-token legend is there;
  it is now `crates/nvs-lsp/src/capabilities.rs:37`'s, so the next session needs the section only to
  overturn it. Add `0099.md § 4` if stage 7 reopens the legend.
