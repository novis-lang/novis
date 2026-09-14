# Handoff

## State

**Goal `m4-refusals` — every shape the checker admits lowers, or a diagnostic naming its rule
refuses it. Stage 2, the keystone, is landed whole.** `lower::guarded_by!` exists, `holes.py
--guarded` lists what it marks, and a gate holds every guard to a conformance case.

- `python tools/holes.py`: **15** refusal sites, `UNATTRIBUTED: 0`, **1** guarded.
  `crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **15** to match.
- The one guarded site is `crates/nvs-ir/src/lower/mod.rs:2636` (`write_back_array`'s catch-all),
  naming `E_ELEMENT_WRITE_ROOT_NOT_A_PLACE`/`E0700`. That is Stage 3's fifth table row, already
  done — its reject case is `tests/conformance/lang/an-element-write-needs-a-place-to-write-back-into.nvst`.
- Which spelling means what has one home: `crates/nvs-ir/src/lib.rs`'s § *Known gaps* preamble.
  A bare `panic!` naming a shape is a hole `holes.py` counts; a `guarded_by!` is a front-end
  guarantee and is not counted.
- Nothing is blocked. Stage 1's floor and the goal's § *Standing decisions* are unchanged.
- **Line numbers in `docs/agent/loop-goal.md` § *Stage 3*'s table are stale by ~22 lines for
  `lower/mod.rs` only** — the macro sits above the `mod` declarations. The other five files are
  untouched, so their rows are current. The anchors below are what `holes.py --sites` reports now.

## Next group

**Stage 3: the guarded sites, six of seven left** — one file set:
`crates/nvs-ir/src/lower/{call,control,expr,stmt}.rs`, plus one reject case per code under
`tests/conformance/`. Probe each shape under `.agent-tmp/` before converting it, per the stage's
own first bullet: a shape that reaches the panic is not guarded and moves to the stage it belongs
to.

- [ ] **The two `foreach` sites** — `crates/nvs-ir/src/lower/control.rs:987` (a subject that is not
      an `array<T>`, guarded by `E0443` at `nvs_types::expr::iteration::report_not_iterable`) and
      `crates/nvs-ir/src/lower/control.rs:976` (a key binding outside `string`, guarded by `E0723`).
      `rule:iteration/foreach-subjects` says anything outside its three subjects is refused at the
      subject, so an un-narrowed `?array<T>` or a `mixed` that gets through is fixed by reporting
      `E0443` there, never by a runtime branch here.
- [ ] **`instanceof` and the by-reference argument** — `crates/nvs-ir/src/lower/expr.rs:5231` (a
      subject that can hold no object, guarded by `E0497`) and
      `crates/nvs-ir/src/lower/call.rs:1171` (a by-reference argument from something that is not a
      place, guarded by `E0439` at `nvs_types::expr::args::check_inout_arg`,
      `crates/nvs-types/src/expr/args.rs:1168`). Each owes a reject case expecting its code.
- [ ] **`unset` and the statement roster** — `crates/nvs-ir/src/lower/stmt.rs:1484` (`unset` on
      anything but a subscripted element, guarded by `E0234` at `check_unset_target`) and
      `crates/nvs-ir/src/lower/stmt.rs:264`, which stops being a catch-all: each shape its
      `:240-262` comment lists as refused upstream gets its own arm naming its own code, so a new
      `StmtKind` fails to compile rather than panicking at run time.

## Backlog

- `docs/agent/loop-goal.md` § *Stage 3*'s table cites `crates/nvs-ir/src/lower/mod.rs:2615` for a
  row now at `:2636` and done — the goal file owns that table.
- `tools/holes.py`'s `--guarded` skips a match behind a `//` on its own line only; a `guarded_by!`
  inside a `/* */` block would read as a site. No such comment exists in either crate today.
- `crates/nvs-ir/src/lib.rs` gap 7 is M12's and the `unowned` rest belong to goal
  `unowned-closures` — not this goal, per § *Standing decisions*.
