# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — authoritative
for the acceptance list and the ten standing decisions already settled with the user; do not re-open any
of them. The plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, and every fixture is
`valgrind --leak-check=full` clean. Five registry mechanisms now work end to end: an **options bag**, a
**union parameter**, a **callback-bound result type**, a **`Core`-owned enum** (`Core\Order`, roster
`mwl_stdlib::registry::ENUMS`) and an **absent option** (`Const::Null` over `mwl_ir::Ty::Null`).
`examples/core.mwl` runs its line 11 sort and stops at `Core\Str::length`. **Nothing is blocked.**

## Next

**Settle [ADR 0009](docs/adr/0009-string-and-bytes.md) by measurement, then `Core\Str::length`/`at`/
`slice`.** The last thing between `examples/core.mwl` and its frozen six lines, and the loop goal's
standing decision spells the whole task out: implement both granularities behind one seam, take the cost
measurement its *Revisiting* asks for, write the figure into
`a_grapheme_index_costs_more_than_a_code_point_index` in `benches/abi-probe/tests/perf_guards.rs` (where
measured numbers live, per CLAUDE.md), pick the default it justifies, and move the ADR to **Accepted**.
`crates/mwl-stdlib/src/str.rs`'s module doc names the two registered members whose answers it changes.

1. **More cheap § 1–2 rows that need nothing new**: `Str::lines`/`chunk`/`replaceAll`, `Arr::reverse`/
   `fill`/`sortByKey`. Each is one registry row plus a body plus a PHP-verified test — `Core\Arr::sort`
   (`b09b6b0`) is that pattern at its most involved, `isList`/`values` (`eff6f01`) at its simplest;
   oracle step and `valgrind` run included either way. `Arr::keys`/`keyOf`/`find`/`first`/`last` are
   **not**: each returns `int|string` or `?T`, the return gap below.
2. Then `crates/mwl-test` and `mwl test` (loop goal, Stage 4).

## Backlog

- **A closure literal written as a call argument leaks its environment object when that call throws** —
  the `landing_block` gap the loop goal names, reproduced: `Core\Arr::filter($a, fn(int $n): bool =>
  Boom::at($n))` inside a `try` leaks one `mwl_object_new`; hoisting the closure to a local first is
  clean. It needs the owned-temporaries stack threaded through `lower_expr`.
- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed; `mwl_types::core_lib`'s module doc owns the trust rule. A
  `Core` **enum case** is out of this hole already: `mwl_types::expr`'s `ClassConstAccess` arm reports
  `E0405` for one the registry does not state, which is the shape a full fix would take.
- **A `Core` member cannot *return* a union or `?T`**, and a `?T` *parameter* cannot be declared at all
  — one `mwl_ir::ty::Ty::Mixed` representation question under both. `mwl_ir::Ty::Null` closed the
  narrower half (the *value* `null`); what is left is a *type* admitting both `null` and a `T`.
- **`Core\Arr::sort` has no natural order for objects** — ADR 0013 says `Comparable::compareTo`, and an
  *instance* call from a helper is not reachable, so it throws naming the interface instead.
- **Only a written `fn` literal binds a callback's result type** — a callable reached through a variable
  leaves `Core\Arr::map`'s `U` at `mixed`. `mwl_types::generics`' own *Known gap* owns it.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5 own both. `a_typed_arithmetic_loop_contains_no_call` counts every
  `call` in `Bench::sum`, so a never-taken raise block needs that guard's claim re-read first.
- **A `Core` helper cannot name the class it throws** — every `Fault::thrown` becomes `RuntimeError`
  (`mwl_runtime::Ctx::set_runtime_error_class`), so spec § 10's `LogicError` is unreachable from
  `mwl-stdlib`. `mwl_runtime::mwl_raise_new` is the shape a fix would reuse.
- **`<`/`>` over two `string`s is unlowered** — `mwl-codegen` refuses a `Gt` over representation `Str`,
  which is why a fixture comparing strings reaches for `Core\Str::startsWith` instead.
- **`$fn(...)`, `for`/`switch`/`match` and ADR 0043's `by`-delegation are all unlowered, and
  `crates/mwl-ir/src/lib.rs`'s module doc is the changelog CLAUDE.md forbids** — `mwl-ir`'s and
  `mwl_types::conformance`'s own docs own all four.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has one home; a second copy is a bug — including this file, overwritten never appended to,
capped at 80 lines. After editing any doc, run `python .claude/brief.py --check`. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. For a multi-line Rust edit, write the old and new blocks to files under `target/` with the Write
  tool, then `python .claude/splice.py <target> <old> <new>` — its own docstring says why.
- **Another agent may be editing this repo at the same time** — one overwrote this very file mid-session.
  **Stage your own paths explicitly, check `git show --stat` after committing, and re-read a shared doc
  immediately before rewriting it.**
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. One fixture:
  `.claude/leak-check.sh <paths>`, whose header says why to run it for **any** new refcount edge.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
