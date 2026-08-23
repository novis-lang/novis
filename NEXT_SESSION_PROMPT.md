# Next session prompt

## State

**M4 and M4S run together as one loop** — read [`.claude/loop-goal.md`](.claude/loop-goal.md) first: it is
authoritative for the acceptance list, for the ten standing decisions already settled with the user, and for
the gaps that sit on the path. Do not re-open any of those decisions. Re-run the Linux leg by hand with
`wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call.

**The object gate is closed.** `mwl_ir::Ty::Object` was the one thing everything else waited on, and it now
works end to end:

- `crates/mwl-runtime/src/object.rs` — a refcounted header plus one uniform 16-byte `Value` slot per
  declared property, ancestors first; an opaque `ClassDesc` whose *address* is the class identity; an
  iterative release sweep. Its module doc is the one home for all four decisions and what each costs.
- `crates/mwl-types/src/layout.rs` — each class's flattened slot order and transitive supertype set,
  published for `mwl-ir` the way `expr_table` already publishes a call's resolved target.
- `mwl_ir::ir::Program::classes` carries that across; `mwl-codegen`'s `Classes` turns it into descriptors
  it bakes into the emitted code as constants.

Verified: `cargo test` green (23 suites), `clippy`/`fmt` clean, nine new end-to-end object tests in
`mwl-codegen`, and the Linux leg agrees byte for byte with **valgrind clean on every fixture that runs**,
including `hooks.mwl`, which now allocates objects.

Four things had to be settled to get there, and each is recorded where it lives — a static method does not
own its receiver (`Lowering::borrowed`); `parent::`/`self::` is an *instance* call when the target is not
`static` (`MethodSig::is_static`); `ResolvedCall::class` is the declaring class, not the receiver's; and a
null payload is MWL's `null`, so every retain/release primitive treats it as a no-op.

## Next: the ordered-hash array with copy-on-write

The other half of M4's Stage 1 gate, and `Core\Arr`'s whole contract rests on it. Nothing in
`examples/arrays.mwl` runs today.

1. **`crates/mwl-runtime/src/array.rs`**, following `object.rs`'s shape: one allocation, a `Cell` refcount,
   ADR 0007 § 5's insertion-ordered string-keyed hash. **Build the refcount-1 in-place check reachable from
   the start** — it is the whole cost argument for
   [ADR 0063](docs/adr/0063-core-api-conventions.md) R3, and
   `a_refcount_one_array_member_mutates_in_place` in `benches/abi-probe` has to measure it. `$a[]` append
   needs the "highest integer key used so far" counter `InstKind::ArrayAppend`'s doc comment describes.
2. **`mwl-codegen`**: `ArrayNew`/`ArrayGet`/`ArraySet`/`ArrayAppend`, plus `Helper::ArrayTruthy`, which has
   no entry point in `mwl-runtime::helpers` yet. `Ty::Array` is already in `Ty::is_refcounted`, so the
   retain/release insertion points already exist — `emit_refcount` just needs its two symbols.
3. **`foreach`** in `mwl-ir` (`lower.rs:865` panics naming it) — `examples/arrays.mwl` and `report.mwl` both
   need it, and it is the first consumer of an array's iteration order.

## Backlog

Ordered roughly by how cheap each is next to the array.

**Now unblocked by the object representation:**

- **The exception surface**, `.claude/loop-goal.md`'s standing decision: spec § 10's `Throwable` tree as
  real classes with readonly properties, replacing `Ty::Throwable`'s opaque runtime value.
  `examples/errors.mwl` needs `LogicError` and `$e->message`. Needs a typed `catch`, which needs
  `instanceof` — `mwl_object_instanceof` exists and is tested; nothing emits it yet.
- **`lower_try` still panics on a second `catch` clause and on `finally`**, and
  `mwl_ir::lower::is_global_throwable` accepts exactly three names.
- **Virtual dispatch.** A call's target is whatever `mwl_types` resolved from the receiver's *static* type,
  so an override reached through a base-typed variable calls the base's. `ClassDesc` is the natural place to
  hang a vtable; nothing needs one yet because no fixture overrides through a base-typed variable.
- **ADR 0014's property hooks.** `examples/hooks.mwl` now runs and prints the raw slot (`0`/`0`/`n=1`
  against a wanted `6`/`20`/`n=6`) — the hook bodies are parsed, checked, and ignored.
- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps on a zero divisor. The throw path
  exists, so this is a checked divisor plus a `Terminator::Throw`. `examples/core.mwl` needs it.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.
- `static`/`self` as a *declared type* (`mwl-ir` panics naming `Atom(StaticTy)`/`Atom(SelfTy)`), and late
  static binding for `new static()`/`static::tag()`. `examples/objects.mwl` and `enums.mwl` both stop here.
- **Interfaces and enums are still skipped by `mwl_ir::lower::lower_file`'s walk**, so an ADR 0043 default
  interface method body is never lowered even though the checker accepts it.

**Independent of it:**

- **A `THROWN` still leaks a temporary in flight** inside the expression that threw
  (`mwl_ir::lower::Lowering::landing_block`); it needs an owned-temporaries stack threaded through
  `lower_expr`. A `FATAL` leaking the frame's locals is deliberate while a `FATAL` ends the request.
- `for` and `switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator ADR 0053's
  generator resumption also wants, so the two pair naturally).
- ADR 0018's `BRANCH` probe — the last of that ADR's three sites still not emitted.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
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

Three tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, and `sed -i` patterns holding a backslash, backtick
  or em-dash silently no-op. Use Write and Edit for any content with escapes.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` first.
- `cargo insta test --accept -p <crate>` is installed, and is how a deliberate lowering change gets its
  snapshots updated. Read the diffs first — a renamed test needs its `.snap` file renamed with `git mv`.
