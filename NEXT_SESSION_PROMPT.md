# Next session prompt

## State

**Milestone M3 is complete** (all eight of [`.claude/loop-goal.md`](.claude/loop-goal.md)'s acceptance
commands pass on Windows and under WSL, byte-identical) and **M4 is under way**. Re-run the Linux leg any
time with `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call (the Bash tool
rewrites the `/mnt/…` path).

**The last session was design, not code.** It settled the shape of the whole standard library and produced
two documents plus one new milestone:

- **[ADR 0063](docs/adr/0063-core-api-conventions.md)** — twenty rules fixing every `Core` member's shape.
  The load-bearing ones: subject-first argument order, one trailing shape-literal options bag instead of
  flag ints, nothing mutates and nothing takes a reference, failure throws while absence is `?T`, and no
  operation is reachable two ways (which closes PHP's procedural-twin-of-a-class duplication axis, its
  mutable/immutable type pairs, and its `from`/`tryFrom` pairs). It also settles the questions the roster
  could not be written without: no ambient timezone, weak digests refused structurally by an enum subset, a
  closed relative-date grammar, `Core\Path` split from `Core\IO` on the capability boundary, scoped output
  capture, three surviving collection types, a small closed exception set, one explicit JSON codec
  interface.
- **[docs/spec/01-core-library.md](docs/spec/01-core-library.md)** — authoritative for every `Core`
  signature, with the PHP built-ins each subsumes and its `tainted`/`secret` classification. ~1,900 PHP
  functions → ~450 members, covering strictly more.
- **New milestone M4S**, between M4 and M4B: implements Part I of that spec (everything pure — no
  capability, no reactor, no driver, no open handle). Part II stays at M8.

Amendments landed with it, all one-liners in illustrative examples: 0007/0011/0027/0031 got the
subject-first flip, 0011 also `Str::len` → `Str::length`, 0051's roster gained four split-out classes, and
0057's intrinsic list was respelled and gained the relative-date grammar.

## Next: continue M4 — the object representation

Read [`docs/implementation-plan.md`](docs/implementation-plan.md)'s **M4** paragraph for scope. Almost
every refusal M3 left behind names one thing, and it is still the gate:

- **An instance method call is refused** (`mwl-codegen` known gap 1). Argument slot 0 is the implicit
  receiver every lowered method carries; a static call fills it with `null`, but a real receiver needs a
  heap layout. `mwl_ir::lower::lower_file` skips interfaces and enums for the same reason.
- **`mwl_ir::Ty::Object` refcounts nothing** — no allocation, no field offsets, no `Tag::Object` payload.
- **A user class `extends Exception` has no runtime shape.** `mwl_ir::lower::is_global_throwable` accepts
  exactly `Throwable`/`Exception`/`Error`, and `lower_try` refuses a `catch` on anything else because
  matching one needs `instanceof`.
- **Arrays** are the same question one step on, paired in the plan with `Throwable::getTrace()`'s
  `array<…>` form (the string form landed in M3; the array form did not).

A reasonable order: object layout and allocation in `mwl-runtime` → `FieldGet`/`FieldSet` and an instance
`Call` in `mwl-codegen` → the ordered-hash array with COW → `getTrace()` and a second `catch` clause.

**The array work now has a consumer.** `Core\Arr`'s entire contract depends on one property M4S will
measure: a member mutates in place when its argument's refcount is 1 and copies when it is not. That is the
whole cost argument for [ADR 0063](docs/adr/0063-core-api-conventions.md) R3, so build the COW array with
that check reachable rather than bolting it on later.

## Backlog

Ordered roughly by how cheap each is next to M4's bulk.

**Rides on the object representation:**

- `finally`, and a second `catch` clause (`mwl_ir::lower::lower_try` panics naming both).
- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps on a zero divisor. The throw
  path exists now, so this is a checked divisor plus a `Terminator::Throw`.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.

**Independent of it:**

- **A `FATAL` still leaks the frame's locals**, and a `THROWN` still leaks a temporary in flight inside the
  expression that threw. Both are stated in `mwl_ir::ir::Inst::on_error` and
  `mwl_ir::lower::Lowering::landing_block`. The second needs an owned-temporaries stack threaded through
  `lower_expr`; the first is deliberate while a `FATAL` ends the request, and should be revisited when
  M5/M6 give a request an arena.
- `for`/`switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator generator
  resumption will also use).
- ADR 0018's `BRANCH` probe — the last of that ADR's three sites still not emitted.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- The safepoint/debug-flags loads use `MemFlagsData::with_notrap()`; they must become atomic when M5
  introduces a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- [0054](docs/adr/0054-decimal-scalar-type.md)'s `m` literal suffix;
  [0053](docs/adr/0053-iteration-and-generators.md)'s `Iterable`/`Iterator` as reserved interface names —
  both now have a consumer in the M4S spec, so they are cheaper to land before it than during it.
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
  content containing escapes — and for `sed -i` substitutions containing backslashes or backticks, which
  silently no-op rather than failing. An em-dash in a match string fares no better.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` into a Git-Bash-relative
  path before `wsl.exe` ever sees it.
