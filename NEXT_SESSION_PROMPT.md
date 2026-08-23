# Next session prompt

## State

**M4 and M4S run together as one loop** — read [`.claude/loop-goal.md`](.claude/loop-goal.md) first: it is
authoritative for the acceptance list, for the ten standing decisions already settled with the user, and for
the gaps that sit on the path. Do not re-open any of those decisions. Re-run the Linux leg by hand with
`wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call.

**Stage 1's array gate is closed end to end.** `examples/arrays.mwl` prints all seven of its frozen lines on
Windows and under WSL against a Linux build, byte for byte, and is valgrind-clean. Three things landed:

- **`foreach` over an `array<T>`** (`mwl_ir::lower::Lowering::lower_foreach`, which owns the whole policy).
  It is `lower_while`'s shape with a synthesized condition, plus three things of its own: the loop retains
  its **own** reference to the array for the loop's duration, which is what makes PHP's by-value `foreach`
  fall out of copy-on-write rather than needing a snapshot; that reference and the cursor live in the
  lowering `Env` under reserved `foreach#N`/`foreach#N$cursor` names (no identifier can contain `#`), which
  is what gets them a loop-header phi and a release on a `return`/throw for free; and the key/value
  bindings are owned for **one iteration**, released at every point an iteration ends —
  `LoopFrame::iteration_owned`, which `lower_break`/`lower_continue` now consult.
- **`unset($a[$k])`**, on `ArraySet`'s consume-one-yield-one protocol, written back through the same
  `write_back_array`. Its key is *borrowed*, which inverts the retain a stored key gets.
- **The whole Tier 0 path**: `crates/mwl-stdlib` (registry + `Core\Arr::count` + `symbols()`),
  `mwl_types::core_lib` seeding the signature table from it, `mwl_types::generics` binding and substituting
  type variables, `mwl_ir::ir::InstKind::CoreCall`, and `mwl-codegen` emitting it through the helper path
  unchanged.

One bug was fixed on the way: `collect_reassigned_locals` only recognised a bare `$x` assignment target, so
a loop body writing an array *element* got no loop-header phi. Invisible while the array was solely owned —
and an infinite loop the moment a `foreach` held the second reference that makes it separate.

## Next: the exception surface, or more `Core` rows

Two independent lines. Pick the one that fits a session; the first is the bigger and the more blocking.

### 1. The exception surface (`examples/errors.mwl`, Stage 1's last unclosed fixture)

`.claude/loop-goal.md`'s standing decision fixes it: **[docs/spec/01-core-library.md](docs/spec/01-core-library.md)
§ 10, not ADR 0020 § 1.** `Throwable` is the root and user classes extend it directly; the tree is
`LogicError`, `RuntimeError` (with `IOError`, `ParseError`, `TimeoutError`) and `ArithmeticError`; members
are **readonly properties** (`$e->message`, `$e->previous`, `$e->backtrace`, `$e->location`), not
`getMessage()` accessors; there is **no `Exception` and no `Error`** class. Amend ADR 0020 § 1 to point at
§ 10 rather than restate it, and migrate `throw.mwl`/`trace.mwl`/`uncaught.mwl`, whose output must stay
byte-identical — that is the point of migrating them.

What it needs, in dependency order:

- **`instanceof`** — `mwl_object_instanceof` exists and is tested; nothing emits it yet.
- **A typed `catch`**, which needs the above, plus `lower_try` handling a **second `catch` clause** and
  `finally` (it panics on both today). `mwl_ir::lower::is_global_throwable` accepts exactly three names.
- Deciding whether `Ty::Throwable`'s opaque runtime value becomes an ordinary `Ty::Object`, now that
  objects have a representation. `mwl-codegen`'s known gap 0 states the choice.
- A **`catch` binding is function-scoped** today, so two clauses on one `try` cannot both bind `$e`
  (`E0406` on the second). PHP allows it. `.claude/loop-goal.md` names this a decide-and-record call, not a
  `BLOCKED`; `errors.mwl` sidesteps it with four distinct names either way.

### 2. More `Core` rows (`examples/core.mwl`, `examples/report.mwl`)

The mechanism is done, so a member is now **one registry row plus one body**. Add rows to
`mwl_stdlib::registry::CLASSES`, a `mwl_helper!` body per row, and an arm in `mwl_stdlib::symbols`.
Read `crates/mwl-stdlib/src/lib.rs`'s module docs first — they own the argument-borrowing rule and why a
`Core` call needs no Cranelift signature of its own.

`report.mwl` needs `Core\Str` and more of `Core\Arr`; `core.mwl` additionally needs closures (`fn`), an
option-bag shape argument, and integer `%`. Registry types not yet expressible: a union (`int|string`), a
nullable (`?T`), a shape, `callable`, `decimal` — each is a `CoreTy` variant plus a `core_lib::lower` arm.

## Backlog

Ordered roughly by how cheap each is.

- **`static`/`self` as a *declared type*** — `mwl_ir::lower_decl_type` panics naming `Atom(StaticTy)`/
  `Atom(SelfTy)`, which is where `examples/objects.mwl` and `enums.mwl` both stop. Cheap: this crate erases
  class identity entirely, so all three spellings are `Ty::Object`. Late static binding for
  `new static()`/`static::tag()` is the real work behind those fixtures.
- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps on a zero divisor. The throw
  path exists, so `%` is a checked divisor plus a `Terminator::Throw`. Note ADR 0007 § 4 makes `int / int`
  return the *union* `int|float`, which is a separate and larger piece than `%`. Two codegen tests use `%`
  as their "an unlowered shape is named, not panicked on" fixture and will need a different one.
- **Virtual dispatch.** A call's target is whatever `mwl_types` resolved from the receiver's *static* type,
  so an override reached through a base-typed variable calls the base's. `ClassDesc` is the natural place
  to hang a vtable.
- **ADR 0014's property hooks.** `examples/hooks.mwl` runs and prints the raw slot (`0`/`0`/`n=1` against a
  wanted `6`/`20`/`n=6`) — the hook bodies are parsed, checked, and ignored.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.
- **Interfaces and enums are still skipped by `mwl_ir::lower::lower_file`'s walk**, so an ADR 0043 default
  interface method body is never lowered even though the checker accepts it.
- **A nested subscript write (`$grid[0][1] = 5`) panics naming itself** in `Lowering::write_back_array`: it
  needs every level of the chain separated and written back in turn. Not on the acceptance path.
- **A `THROWN` still leaks a temporary in flight** inside the expression that threw
  (`mwl_ir::lower::Lowering::landing_block`); it needs an owned-temporaries stack threaded through
  `lower_expr`. A `FATAL` leaking the frame's locals is deliberate while a `FATAL` ends the request.
  Related and narrower: a `foreach` inside a `try` whose body throws carries its two reserved `Env` names
  into the handler only if every landing edge has them — see `lower_foreach`'s own doc comment.
- `for` and `switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator ADR 0053's
  generator resumption also wants, so the two pair naturally).
- **`foreach (… as &$v)`** panics naming itself: a by-reference value binding writes back through the array
  it is walking, the one shape copy-on-write has to be told *not* to separate.
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
  `every_part_one_member_has_a_conformance_case` is now cheap to write: it can read the registry.
- **Docs-only:** `crates/mwl-ir/src/lib.rs`'s module doc has become a slice-by-slice changelog of exactly
  the kind CLAUDE.md forbids. It is the one doc in the repo genuinely owed a trim pass.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`: it enforces a per-entity cap and names the
exact line and byte count to cut. That is a one-line fix, never a reason to run a trim pass.

Four tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc** — including inside a `python - <<'PY'` script, and
  including a `\\` inside a `cat >> file <<'RS'` block, which silently becomes a single backslash and
  breaks the Rust string literal it lands in. Write a Python helper to a *file* with the Write tool and run
  the file; use Write/Edit for any content with escapes.
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` first.
- `cargo insta test --accept -p <crate>` is installed (note `test --accept`, not `accept -p`, which does
  not take `-p`), and is how a deliberate lowering change gets its snapshots updated. Read the diffs first
  — a renamed test needs its `.snap` file renamed with `git mv`.
