# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `python tools/owners.py --closes decided-closures`
names **5**, down from 7: `regex.rs` gap 1 is struck and `ast.rs` gap 1 is built. The other two
stage-6 gates stay green (`playbook.py --closes` names no row, `owners.py --check
--past-is-an-error` reports `past-milestone: 0`).

**`regex.rs` gap 1 struck as a stated bound, and a deferral was not available.** The sheet's
disposition for it (`docs/agent/loop-goal.md:110`) reads it as gap 3 and sends it to **M6**, a
milestone the program has passed, and no plan file at M9 or later states per-request allocation
provenance or accounting-bracket scope — checked across `docs/plan/m9.md`, `m10`, `m11`, `m12`,
`m15`, `m16`, `m17`. So the module's own prose now carries what it costs: the request whose write
clears the cache is credited with the programs earlier requests paid for, bounded each time by
`CACHE_CAPACITY`, and the pattern text is a sink taking the plain `string`
(`rule:security/regex-pattern-is-a-sink`) so the cached set is the program's own.
`crates/nvs-stdlib/src/cldr.rs:232` pointed at "`crate::regex`'s own known gap 1" and now names that
section instead.

**`ast.rs` gap 1 built: a node answers `line()`, `column()` and `offset()`.**
`nvs_syntax::walk::located` keeps the `SourceMap` it parsed against and `Located::position` resolves
a node's span start through `nvs_diagnostics::SourceFile::line_col`, so position arithmetic stays in
its one home (`rule:ide/positions-have-one-home`) and `nvs-stdlib` gains no line counting of its own.
`of_source` is that call with the map dropped. The three values are slots on the node, filled at
parse time; `rule:core-classes/ast-is-inert` gained the paragraph saying a node says **where** it is
and never **what** it says, which is why `parse`'s `$source` stays `Qual::Neutral`. The inertness
sweep in `crates/nvs-stdlib/src/ast.rs:786` now admits a scalar return and asserts the three slots
are `Tag::Int`.

## Next group

**Stage 4: `Core\Command`'s help page and the table row it reads** — one file set:
`crates/nvs-stdlib/src/command.rs` and `crates/nvs-types/src/commands.rs`.

- [ ] **Check the dependency before building — `crates/nvs-types/src/commands.rs:125`.** That module
      doc cites "the module's gap 1" at `:125`, `:132`, `:594` and `:656`, but the module has no
      `# Known gaps` section and `python tools/owners.py --closes decided-closures` owns nothing
      there, so the blocker `command.rs` names may already be closed. The playbook's *A crate's
      known-gap bullet can give a reason that is false about the driver it names* is this exact
      shape; settle it first, because the answer decides whether the next item is one slice or two.
- [ ] **`crates/nvs-stdlib/src/command.rs:74` — gap 1, a build.** `Decided: The help renderer reaches
      the handler's signature at render time (row holds a reference, not a copy)`. A page names no
      types today because `nvs_runtime::commands::CommandArg` carries a declared default and not a
      declared type; the row needs a way to point at the signature the handler already holds, and a
      second copy of it in the table is what the sibling gap refuses.
      `rule:tooling/commands-are-compiled` is the rule it is written against.

## Backlog

- `crates/nvs-diagnostics/src/embedded.rs:30` gap 1 — `autoload` probing, this goal's (stage 4).
- `crates/nvs-syntax/src/lib.rs:96` gap 1 — a local typed with a bare inline shape, this goal's.
- `crates/nvs-types/src/defaults.rs:58` gap 1 and `crates/nvs-types/src/response.rs:29` gap 1 — one
  file set of their own, both this goal's.
- `docs/agent/loop-goal.md:110`'s gap numbering is one behind `regex.rs`'s: the sheet's gaps 2 and 3
  are the module's landed step-budget directive and the cache struck here. Read a sheet row by what
  it describes, not by its number.
