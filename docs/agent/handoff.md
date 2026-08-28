# Handoff

## State

**M4's frontier past the two file-scope statement shapes.** Both are landed: an inline-HTML run's
placement (last session) and now a `require`d file's own top-level statements, which was
`nvs-ir` known gap 22's statement half and a three-crate change. `holes.py --cases` should name
neither.

- **`require` runs the target's top-level statements at the site.** The mechanism is one sentence
  in three homes and is not restated here: `nvs_hir::Loaded::requires` (the `span -> SourceId`
  edge, and why only the walk can produce it), `nvs_types::ExprTypeTable::record_require_target`
  (why it rides in that table rather than in a `lower_program` argument), and
  `nvs_ir::lower::file_script_label` (the per-file frame). `nvs-ir`'s gap 22 now records only what
  is left.
- **The decision this session took, recorded rather than left open**: declarations cross a
  `require` and variables do not. ADR 0021 § *Decision* now says so in its own paragraph, with the
  PHP divergence named. `nvs_types::locals` had already made it true; the ADR body was the thing
  disagreeing.
- **`nvs_types::ProgramFile` deliberately gained no `id` field** — `src.id()` is the file's own id
  and always was. Its doc comment carries that, so a future session does not add the copy.
- **An autoloaded file now gets a script frame nothing calls.** It is correct (no statement reaches
  such a file, so nothing of its body should run) but it is a compiled, unreachable function per
  autoloaded file. Backlogged rather than fixed: skipping it wants `lower_program` to know which
  files the walk reached by `require` and which by the map, which is one more edge out of
  `nvs-hir`.
- **`orient.py`'s pack was complete for this item.** The two standing manifest gaps are unchanged —
  `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` entry.

## Next group

**ADR 0021 § 3's value form — `$c = require 'config.nvs';`, gap 22's other half — in three slices
over one file set.** The files: `crates/nvs-types/src/expr/mod.rs`,
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/stmt.rs`.

- [ ] **A `return expr;` at a required file's file scope returns it from that file's frame.**
      `lower_script` (`crates/nvs-ir/src/lower/mod.rs:1010`) seals with `Terminator::Return(None)`
      against a declared `Ty::Tagged`; `lower_script_stmts`
      (`crates/nvs-ir/src/lower/stmt.rs:20`) is where a file-scope `return` arrives.
      `nvs_types::check::check_program` already types that frame's return as `mixed` — its
      `ScriptFrame::return_ty` says so — so this is the lowering half alone.
- [ ] **The `require` site reads the value.** `crates/nvs-ir/src/lower/stmt.rs:340` already emits
      the call and discards its `Ty::Tagged`; the expression form needs the same call in
      `lower_expr` and the refusal at `crates/nvs-types/src/expr/mod.rs:743`
      (`E_REQUIRE_VALUE_UNLOWERED`, `E0704`) retired with it. `E0704`'s own constant at
      `crates/nvs-diagnostics/src/lib.rs:1216` goes too — a retired code is never reused.
- [ ] **`tests/conformance/lang/a-required-file-hands-a-value-back.nvst`** — the value of a file
      that returns, of one that does not (`null`, ADR 0021 § 3's `mixed`), and the same `require`
      reached twice returning twice.

## Backlog

- An autoloaded file's uncalled script frame is compiled dead code — `nvs-ir` `lower_program`.
- `[context] modules` still names no `nvs-runtime` and no `nvs-diagnostics` — `loop-goal.toml`.
- `python tools/gaps.py` for the next depth-case group once gap 22 closes — `docs/agent/commands.md`.
- ADR 0024 § 5's `string as Core\Html\Markup` waits on `Core\Html` (M7) — `nvs-ir` catch-all roster.
