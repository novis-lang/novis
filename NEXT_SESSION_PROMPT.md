# Next session prompt

## State

**Read [`.claude/loop-goal.md`](.claude/loop-goal.md) in full before anything else** — it is the
authoritative scope for this run and holds decisions the user already made, so nothing below re-opens
them. The goal is **all of M3**: `docs/implementation-plan.md`'s M3 *Verify* bullet, machine-checked by
an acceptance list that runs on Windows **and** WSL.

`examples/*.mwl` are the acceptance fixtures and are **frozen** — changing one is a `BLOCKED`, not an edit.

**Five of the eight `mwl run` acceptance commands now pass**, on Windows and — checked by hand this
session, byte for byte — under WSL against a Linux build: `hello`, `calls`, `arith`,
`--fault-inject=helper-panic fatal`, `--dump-asm arith`. Both `cargo test` legs the list names are green,
including all three tests it requires by name. `cargo deny check` is green. Build, test, clippy and fmt
are green.

Landed this session, one commit each: the paired wasmtime 41→48 / Cranelift 0.128→0.135 bump (three
mechanical API changes, `benches/abi-probe` re-run including the wasm probes); `mwl run --dump-asm`;
`.` concatenation on a new `mwl_str_concat` primitive; a compiled static MWL call end to end; ADR 0018's
call-site `TRACE`/`PROFILE` probe pair; `--fault-inject`; and the two typed-arithmetic guards.

## Next

**The three remaining acceptance commands — `throw.mwl`, `trace.mwl`, `uncaught.mwl` — are one piece of
work, and it is all that is left of M3.** Nothing throws today: `mwl-ir` models no error edge,
`try`/`catch`/`throw` are lowered nowhere, and there is no runtime `Throwable`. All three examples fail
in `mwl-ir` lowering with *"only lowers a plain `$x = expr;` reassignment or a bare call/`new`"*.

The goal file's standing decisions already settle the shape — read them rather than re-deciding — and the
pieces are:

1. **`mwl-runtime`: the `Throwable` value.** Runtime-owned and opaque, not a user class:
   `new Exception("…")` lowers to a runtime helper that allocates it, and `getMessage()`/
   `getTraceAsString()` are runtime methods on it, never general field access. `Ctx::pending` already
   carries a `Cow<str>` message for `THROWN`; decide whether the `Throwable` subsumes it or sits beside
   it, and record that in `mwl-runtime`'s own module doc.
2. **`mwl-ir`: the error edge.** The crate's module doc lists `try`/`catch` as absent alongside
   `for`/`switch`; unlike those two it is on the path. Every call-shaped instruction (`Call`,
   `HelperCall`, `Concat`'s helper operands) needs an edge to a cleanup/landing block. The goal file
   makes `mwl-codegen`'s known gap 3 — releasing the frame's live refcounted locals before propagating —
   land **with** this, not after.
3. **`mwl-codegen`: `Throw`, `try`/`catch`, and the cleanup path.** `emit_status_check` today returns the
   status onward from a bare `fail` block; that block becomes the site of both the refcount cleanup and,
   inside a `try`, the branch to the `catch` landing block.
4. **The backtrace.** Shape and source are fixed by the goal file: `#0 Class::method() at <file>:<line>`,
   from MWL's own frame chain, positions from the per-statement ids already in the IR. **Suggested
   mechanism, not yet decided:** build the trace *as the throw propagates* — each frame's error path calls
   a `mwl_trace_push(ctx, ptr, len)` with a static `(name, file, line)` label emitted into the unit's data
   section. That costs nothing on the success path, which is ADR 0002's whole point, and reuses the
   pointer/length-into-the-data-section technique `emit_call_probe`/`emit_bytes` already established this
   session. A push/pop frame record on every call would cost the success path instead — weigh it, pick
   one, record it in the crate's own module doc.
5. **`mwl-cli`** already spells `Uncaught Exception:` and `FATAL:` for the two statuses; it needs to print
   the backtrace after the message for the uncaught case.

Once those land, run the WSL leg the same way this session did: build with `CARGO_TARGET_DIR=/tmp/mwl-linux`
and run the eight commands through `wsl.exe -- bash <script>`. Note that a `wsl.exe -- bash -lc "…"`
one-liner mangles under the Bash tool's quoting — write the script to a file and pass the path.

## Backlog

- `for`/`switch` in `mwl-ir` (reuses `LoopFrame`; `switch` needs the N-way terminator generator
  resumption will also use). Off-path — `examples/arith.mwl` uses `while` deliberately.
- Integer `Div`/`Mod`, refused in `mwl-codegen` because `sdiv` traps the process on a zero divisor.
  Fixable once exceptions work; not acceptance.
- **An instance method call is refused** (`mwl-codegen` known gap 1): argument slot 0 is the implicit
  receiver every lowered method carries, and a static call fills it with `null`, but a real receiver needs
  M4's object representation. `lower_file` skips interfaces and enums for the same reason.
- A `throw` must satisfy the return check — a method declared `: int` whose body always throws counts as
  returning. The acceptance examples dodge it by declaring the throwing chain `void`.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- The safepoint/debug-flags loads use `MemFlagsData::with_notrap()`; they must become atomic when M5
  introduces a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix;
  [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as reserved interface names.
- **Docs-only, optional:** `CLAUDE.md` is read in full every session, and most of it is two lists that
  each grow one entry per ADR — the *Where to look* table and *Ground rules enforced elsewhere*. Putting
  both under the same per-entity cap `brief.py --check` already enforces elsewhere would roughly halve the
  per-ADR growth without deleting anything. Not started; not on M3's path.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`: it enforces a per-entity cap and names the
exact line and byte count to cut. That is a one-line fix, never a reason to run a trim pass.

One tooling note worth keeping: **the Bash tool eats a backslash inside a heredoc**, so `\n` written into
a Python or shell heredoc reaches the file as a real newline and breaks the Rust source. Use the Write and
Edit tools for any content containing escapes — CLAUDE.md already says to, and this is the failure mode it
is protecting against.
