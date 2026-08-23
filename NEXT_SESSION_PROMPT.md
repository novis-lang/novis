# Next session prompt

## State

**A new loop just started. Read [`.claude/loop-goal.md`](.claude/loop-goal.md) in full before anything
else** — it is the authoritative scope for this run and it holds decisions the user already made, so
nothing below re-opens them. The goal is **all of M3**, not the vertical slice: `docs/implementation-plan.md`'s
M3 *Verify* bullet, machine-checked by an eight-command acceptance list that runs on Windows **and** WSL.

On disk: `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-runtime`, `mwl-codegen`,
`mwl-cli` (`ast`, `check`, `run`, `run --dump-ir`), `fuzz/`, `benches/abi-probe`. Build, test, clippy and
fmt are green; `cargo deny check` is **not** (see *Next*). `mwl run examples/hello.mwl` prints from natively
compiled code; every other acceptance example fails today, the first at
`mwl-codegen does not lower a resolved method call yet`.

`examples/*.mwl` are the acceptance fixtures and are **frozen** — changing one is a `BLOCKED`, not an edit.

## Next

**Bump wasmtime and Cranelift together, and get `cargo deny check` green — before any M3 work.** The goal
file says why this is first and what makes it a paired bump rather than two version edits: 13 RUSTSEC
advisories want wasmtime ≥ 46, and the pin at 41 exists only because M0's spike #4 proved it coexists with
Cranelift 0.128. Re-run that spike and every `benches/abi-probe` guard. If Cranelift's API churned, fixing
`mwl-codegen` against the new version is part of this task — and it is far cheaper now than after the
backend grows calls, exceptions and a backtrace.

Then, in this order, because each unblocks the next:

1. `mwl run --dump-asm` — smallest item on the list, depends on nothing, `--dump-ir` is the model (it
   prints *instead of* running). Good first commit after the bump.
2. **Lower `InstKind::Call` and `Concat`.** Needs a symbol table over the unit's own functions
   (`InstKind::Call::target` is a `"Class::method"` label) and `mwl-cli`'s `run_run` lowering a file's
   methods, not just `lower_script`. ADR 0018's call-site probe lands in `emit_call`'s single path *with*
   this, per the goal file — not as a later slice.
3. **The error edge and `try`/`catch`.** `mwl-ir` models no error edge at all; the goal file makes the
   refcount cleanup land with it rather than after. `throw`/`catch` and the runtime-owned `Throwable` follow.

## Backlog

- `for`/`switch` in `mwl-ir` (reuses `LoopFrame`; `switch` needs the N-way terminator generator resumption
  will also use). Off-path — `examples/arith.mwl` uses `while` deliberately.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.
- Integer `Div`/`Mod`, refused in `mwl-codegen` because `sdiv` traps the process on a zero divisor. Fixable
  once exceptions work; not acceptance.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- The safepoint/debug-flags loads use `MemFlags::with_notrap()`; they must become atomic when M5 introduces
  a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix;
  [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as reserved interface names.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
Doc trimming is authorized **only** when `python .claude/brief.py` reports a truncated section, and then
only for that one doc — see the goal file.
