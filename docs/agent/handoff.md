# Handoff

## State

**M4 — language completeness.** `exit` and `exit(...)` run, end to end. `mwl_runtime::EXITED`
is a fourth ABI status beside `OK`/`THROWN`/`FATAL` (`crates/mwl-runtime/src/abi.rs:37`) and
the status the program named rides out on `Ctx::exit_code`. The construct lowers to a single
`Helper::Exit` call (`crates/mwl-ir/src/lower/expr.rs:@lower_exit`) whose *success* is that
non-`OK` status, so the ADR 0002 status check `mwl-codegen` already emits takes the site's
error edge: the frame's live locals are released in its landing block, and every caller's own
check propagates it onward for nothing.

**No `catch` sees it and no `finally` runs** — `Terminator::Catch` admits only `THROWN`, and
every copy of a `finally` body lives behind that comparison. That is PHP's own behaviour,
checked against `php -r` rather than assumed; the item that scheduled this work asserted the
opposite, and the playbook now says so. `docs/adr/README.md` § *Decisions taken at project
start* owns the decision and what it costs.

The operand carries both of PHP's spellings: an `int` is the process status, a `string` is a
message written first with the status left at `0`. Anything else is `E0401` at the operand
(`crates/mwl-types/src/expr/mod.rs:496`) — no new diagnostic code was claimed, because
**`E0499` is still the last code in the `E04xx` band** and a band decision is owed before the
next types diagnostic. `mwl run` maps `EXITED` to the low byte of the code and reports nothing.

`verify.py` 6 of 6 green — conformance **614**, differential 173. `tools/leak-check.sh` clean
over a fixture that `exit(7)`s out of a nested frame holding a `string` local.

## Next group

**`object` as a declared type, in the two crates that erase it.** They share the
representation map: `crates/mwl-ir/src/lower/mod.rs` and `crates/mwl-codegen/src/ty.rs`. The
standing decisions already settle the design — "`object` erases to the same pointer a named
class does" — so what is left is the two erasure arms and the two codegen rows, plus finding
whether anything below reads a class label.

- [ ] **`erase_checked_ty` has no `object` arm.** `crates/mwl-ir/src/lower/mod.rs:2206` is the
      map; the two refusals are `crates/mwl-ir/src/lower/mod.rs:2399` (a declared type) and
      `crates/mwl-ir/src/lower/mod.rs:2482` (a resolved call's parameter or return). Watch the
      playbook's "two edits, and the second one panics somewhere else" bullet — it is this
      exact function.
- [ ] **`mwl-codegen`'s representation map refuses the same two shapes.**
      `crates/mwl-codegen/src/ty.rs:116` ("the static tag of a tagged value") and
      `crates/mwl-codegen/src/ty.rs:121` ("a value of representation `{ty:?}` crossing a call
      boundary") are the sites `holes.py` reports as attributed to no item at all. Whatever
      `object` erases to above has to land here, or the feature fails one crate later.

## Backlog

- `static::$prop` is `E0499` rather than PHP's called-class resolution — the slot layout keys
  on the declaring class; `docs/adr/README.md` § *Decisions taken at project start*.
- **`E0499` is the last `E04xx` code.** The next types diagnostic needs a band decision in
  `crates/mwl-diagnostics/src/lib.rs`; this session sidestepped it by reusing `E0401`.
- `exit` has no `tests/differential/` case pinning the `finally` skip against PHP directly —
  the agreement was checked by hand and is pinned only in `tests/conformance/lang/`.
- Item 1, ADR 0007 § 4's promotion table: 11 refusal sites, the largest single item left
  (`python tools/holes.py --item 1`).
- Item 7, `$x++`/`--$x`: 5 sites. Item 16, a named or spread argument: 1 site, checker half
  first per the standing decisions.
- `tests/conformance/` still owes 17 of the 32 named cases (`python tools/holes.py --cases`).
