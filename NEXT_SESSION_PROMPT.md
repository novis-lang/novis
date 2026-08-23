# Next session prompt

## State

**Milestone M3 is complete.** All eight of [`.claude/loop-goal.md`](.claude/loop-goal.md)'s acceptance
commands pass, on Windows and under WSL against a Linux build, with byte-identical output on both:

| command | result |
|---|---|
| `mwl run examples/hello.mwl` | `Hello, World!` |
| `mwl run examples/calls.mwl` | `quadruple(5) = 20` |
| `mwl run examples/throw.mwl` | `caught: boom` |
| `mwl run examples/arith.mwl` | `sum = 998000` |
| `mwl run examples/trace.mwl` | `#0 Deep::inner() at …:4` then `#1 Deep::outer() at …:8` |
| `mwl run examples/uncaught.mwl` | exit 1, `Uncaught Exception: unhandled` + three backtrace frames |
| `mwl run --fault-inject=helper-panic examples/fatal.mwl` | exit 1, `FATAL`, `start` still on stdout |
| `mwl run --dump-asm examples/arith.mwl` | ~8.9 KB of generated code |

`cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`,
`cargo deny check` and `cargo test --release -p mwl-abi-probe` are all green, the last including both
typed-arithmetic guards. Re-run the Linux leg any time with
`wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call (the Bash tool rewrites
the `/mnt/…` path).

The throw work that closed M3, in one commit each: the runtime `Throwable` and its primitives; `mwl-ir`'s
error edge plus `throw`/`try`/`catch` lowering; `mwl-codegen`'s landing blocks and the CLI's backtrace
report. Each crate's own module doc holds the design — do not re-derive it from the diff.

## Next: start M4

Read [`docs/implementation-plan.md`](docs/implementation-plan.md)'s **M4** paragraph for the scope. It is a
~10-week milestone, so the first session's real job is to pick the one thing everything else waits on and
land it.

**That thing is the object representation.** Almost every refusal M3 left behind names it:

- **An instance method call is refused** (`mwl-codegen` known gap 1). Argument slot 0 is the implicit
  receiver every lowered method carries; a static call fills it with `null`, but a real receiver needs a
  heap layout. `mwl_ir::lower::lower_file` skips interfaces and enums for the same reason.
- **`mwl_ir::Ty::Object` refcounts nothing** — no allocation, no field offsets, no `Tag::Object` payload.
- **A user class `extends Exception` has no runtime shape.** `mwl_ir::Ty::Throwable` is the runtime's own
  opaque value; `mwl_ir::lower::is_global_throwable` accepts exactly `Throwable`/`Exception`/`Error`, and
  `lower_try` refuses a `catch` on anything else because matching one needs `instanceof`.
- **Arrays** are the same question one step on, and the plan pairs them with `Throwable::getTrace()`, which
  M3 recorded as an explicit carry-over (the string form landed; the `array<…>` form did not).

A reasonable order: object layout and allocation in `mwl-runtime` → `FieldGet`/`FieldSet` and an instance
`Call` in `mwl-codegen` → the ordered-hash array with COW → `getTrace()` and a second `catch` clause.

## Backlog

Ordered roughly by how cheap each is next to M4's bulk.

**Rides on the object representation:**

- `finally`, and a second `catch` clause (`mwl_ir::lower::lower_try` panics naming both).
- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps the process on a zero divisor.
  The throw path now exists, so this is a checked divisor plus a `Terminator::Throw` — it just needs the
  divisor check emitted before the division.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning. The acceptance examples dodge it by declaring the throwing chain `void`.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.

**Independent of it:**

- **A `FATAL` still leaks the frame's locals**, and a `THROWN` still leaks a temporary in flight inside the
  expression that threw. Both are stated in `mwl_ir::ir::Inst::on_error` and
  `mwl_ir::lower::Lowering::landing_block` rather than hidden. The second needs an owned-temporaries stack
  threaded through `lower_expr`; the first is deliberate while a `FATAL` ends the request anyway, and
  should be revisited when M5/M6 give a request an arena.
- `for`/`switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator generator
  resumption will also use).
- ADR 0018's `BRANCH` probe — the last of that ADR's three sites still not emitted; it needs a per-edge
  site at `Terminator::Branch`'s lowering.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- The safepoint/debug-flags loads use `MemFlagsData::with_notrap()`; they must become atomic when M5
  introduces a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix;
  [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as reserved interface names.
- **Docs-only:** `crates/mwl-ir/src/lib.rs`'s module doc has become a slice-by-slice changelog of exactly
  the kind CLAUDE.md forbids. It is the one doc in the repo genuinely owed a trim pass.
- **Docs-only, optional:** `CLAUDE.md` is read in full every session, and most of it is two lists that each
  grow one entry per ADR — the *Where to look* table and *Ground rules enforced elsewhere*. Putting both
  under the same per-entity cap `brief.py --check` already enforces elsewhere would roughly halve the
  per-ADR growth without deleting anything.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`: it enforces a per-entity cap and names the
exact line and byte count to cut. That is a one-line fix, never a reason to run a trim pass.

Two tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, so `\n` written into a Python or shell heredoc
  reaches the file as a real newline and breaks the Rust source. Use the Write and Edit tools for any
  content containing escapes. An em-dash in a match string fares no better — an `assert old in s` that
  fails on text you can see in the file is usually this.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` into a Git-Bash-relative
  path before `wsl.exe` ever sees it.
