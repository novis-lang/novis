# Next session prompt

## State

**M4 and M4S run together as one loop** — read [`.claude/loop-goal.md`](.claude/loop-goal.md) first: it is
authoritative for the acceptance list, for the ten standing decisions already settled with the user, and for
the gaps that sit on the path. Do not re-open any of those decisions. Re-run the Linux leg by hand with
`wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call.

**Both Stage 1 representations are closed.** The array landed this session, end to end:

- `crates/mwl-runtime/src/array.rs` — ADR 0007 § 5's insertion-ordered, string-keyed hash, refcounted and
  copy-on-write. Its module doc is the one home for its four decisions: the container is opaque to compiled
  code and therefore ordinary safe Rust; the index map keeps std's keyed SipHash rather than a fast
  non-keyed one, because array keys are where attacker-controlled bytes become hash inputs; deletion leaves
  a tombstone with amortized compaction; and **every mutating primitive consumes one reference to its array
  and returns one**, which is the only shape a separation can take when a backend keeps a local in an SSA
  register.
- `crates/mwl-runtime/src/release.rs` — new, and shared. An array can hold an object that holds an array, so
  a per-kind worklist would only move the recursion to the boundary between them. `object::dismantle` and
  `array::dismantle` both hand their children to that one drain.
- `mwl_ir::InstKind::ArraySet`/`ArrayAppend` now **define a value**, and `Lowering::write_back_array`
  re-points the holder — a local's `Env` binding or the property slot a `FieldSet` writes back into — with
  no retain and no release, because the consumed reference and the produced one are the holder's same slot.
- `mwl-codegen` compiles `ArrayNew`/`ArrayGet`/`ArraySet`/`ArrayAppend`, `Helper::ArrayTruthy` and a
  `Ty::Array` retain/release. A `Value` crosses a primitive boundary through a caller-owned 16-byte stack
  slot, never in registers — a struct that size is classified differently by the SysV and Windows x64 ABIs.

Verified: `cargo test` green, `clippy`/`fmt` clean, six new end-to-end array tests in `mwl-codegen`
(including copy-on-write separation and a 10 000-write in-place loop that leaks nothing),
`a_refcount_one_array_member_mutates_in_place` measuring 25 ns in place against 34 µs separating, and the
Linux leg agreeing with Windows fixture for fixture with valgrind clean on everything that runs.

## Next: `foreach`, then `unset($a[$k])`

`examples/arrays.mwl` and `examples/report.mwl` both stop at `foreach` and nothing else does.
`mwl_ir::lower` panics naming it at `lower.rs:916`.

1. **`foreach` in `mwl-ir`.** The runtime side is already built and unit-tested: `mwl_array_next_slot`
   (returns `-1` when exhausted), `mwl_array_key_at` (+1 retained) and `mwl_array_value_at` (borrowed, into
   a caller slot). A cursor rather than a borrowed iterator on purpose — the loop holds its own reference,
   so a write inside the body separates and the cursor keeps walking the snapshot the loop started on,
   which is exactly PHP's by-value `foreach`. Reuse `LoopFrame` the way `while` does; the key/value
   bindings are ordinary locals whose refcounting `bind_local` already covers.
2. **`unset($a[$k])`.** No `InstKind` exists for it; `mwl_array_unset` does, on the same
   consume-one-return-one protocol as `ArraySet`, so lowering reuses `write_back_array` unchanged.
   `examples/arrays.mwl` needs it, and ADR 0028 already fixes that `unset` on a declared *property* is a
   diagnostic — this is only the array-element form.
3. **`foreach` over an array of arrays** is the second half of `arrays.mwl` (`$grid`), so nested loops have
   to work before that fixture's `total=10` line does.

After that, `arrays.mwl` still needs `Core\Arr::count` — the first `mwl-stdlib` member, whose entry point
(`mwl_array_count`) is already exported.

## Backlog

Ordered roughly by how cheap each is.

- **The exception surface**, `.claude/loop-goal.md`'s standing decision: spec § 10's `Throwable` tree as
  real classes with readonly properties, replacing `Ty::Throwable`'s opaque runtime value.
  `examples/errors.mwl` needs `LogicError` and `$e->message`. Needs a typed `catch`, which needs
  `instanceof` — `mwl_object_instanceof` exists and is tested; nothing emits it yet.
- **`lower_try` still panics on a second `catch` clause and on `finally`**, and
  `mwl_ir::lower::is_global_throwable` accepts exactly three names.
- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps on a zero divisor. The throw path
  exists, so this is a checked divisor plus a `Terminator::Throw`. `examples/core.mwl` needs it, and two
  codegen tests now use `%` as their "an unlowered shape is named, not panicked on" fixture — they will
  need a different one.
- **Virtual dispatch.** A call's target is whatever `mwl_types` resolved from the receiver's *static* type,
  so an override reached through a base-typed variable calls the base's. `ClassDesc` is the natural place to
  hang a vtable.
- **ADR 0014's property hooks.** `examples/hooks.mwl` runs and prints the raw slot (`0`/`0`/`n=1` against a
  wanted `6`/`20`/`n=6`) — the hook bodies are parsed, checked, and ignored.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.
- `static`/`self` as a *declared type* (`mwl-ir` panics naming `Atom(StaticTy)`/`Atom(SelfTy)`), and late
  static binding for `new static()`/`static::tag()`. `examples/objects.mwl` and `enums.mwl` both stop here.
- **Interfaces and enums are still skipped by `mwl_ir::lower::lower_file`'s walk**, so an ADR 0043 default
  interface method body is never lowered even though the checker accepts it.
- **A nested subscript write (`$grid[0][1] = 5`) panics naming itself** in `Lowering::write_back_array`: it
  needs every level of the chain separated and written back in turn. Not on the acceptance path.
- **A `THROWN` still leaks a temporary in flight** inside the expression that threw
  (`mwl_ir::lower::Lowering::landing_block`); it needs an owned-temporaries stack threaded through
  `lower_expr`. A `FATAL` leaking the frame's locals is deliberate while a `FATAL` ends the request.
- `for` and `switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator ADR 0053's
  generator resumption also wants, so the two pair naturally).
- ADR 0018's `BRANCH` probe — the last of that ADR's three sites still not emitted.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- **An array header carries no interned element-type descriptor** (ADR 0007 § 5). Nothing reaches an array
  through `mixed`, `json_decode` or an isolate boundary yet, so it is a widening rather than a redesign —
  named in `mwl-runtime::array`'s own known gap.
- The safepoint/debug-flags loads use `MemFlagsData::with_notrap()`; they must become atomic when M5
  introduces a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- `crates/mwl-test` and the `mwl test` subcommand — Stage 4's two suites hold most of this loop's coverage,
  and hand-writing them as PowerShell assertions instead is the trap. Worth starting before the suites grow.
- **Docs-only:** `crates/mwl-ir/src/lib.rs`'s module doc has become a slice-by-slice changelog of exactly
  the kind CLAUDE.md forbids. It is the one doc in the repo genuinely owed a trim pass.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`: it enforces a per-entity cap and names the
exact line and byte count to cut. That is a one-line fix, never a reason to run a trim pass.

Four tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc** — including inside a `python - <<'PY'` script, where
  a `\\` in a Rust raw string turns into a line continuation and silently breaks the match. Use Write and
  Edit for any content with escapes, and write a Python helper to a *file* before running it.
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` first.
- `cargo insta test --accept -p <crate>` is installed, and is how a deliberate lowering change gets its
  snapshots updated. Read the diffs first — a renamed test needs its `.snap` file renamed with `git mv`.
