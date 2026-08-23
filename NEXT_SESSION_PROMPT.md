# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — authoritative
for the acceptance list and for the ten standing decisions already settled with the user; do not re-open
any of them. The plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, and every fixture is
`valgrind --leak-check=full` clean. ADR 0063 R2's **options bag** and a **union parameter** on a `Core`
member both work end to end (`crates/mwl-stdlib/src/registry.rs`'s module docs own both designs).
`examples/core.mwl` reaches its **line 9**. **Nothing is blocked.**

## Next

**`Core\Arr::map`, and the unbound-`U` decision under it.** `map(array<T> $a, callable $fn): array<U>`
cannot bind `U` today, because ADR 0027 § 2 keeps `callable` opaque and `mwl_types::generics`' own
*Known gap* says so; `U` substitutes to `mixed`, which `Core\Str::join` rightly refuses, and that is
`examples/core.mwl` line 8. The narrow fix that stays inside the loop goal's "type variables stay
compiler-owned" decision: a `Ty` that names **only the callback's result type**, spellable only from the
registry, plus one arm in `check_generic_args` that binds it from the `ExprInfo::Closure { return_ty }`
the checker already records at the `fn` literal's own span. It constrains nothing (a `callable` value
still satisfies it — that is ADR 0027 § 2), so nothing else in `is_assignable` changes. A callback that
is *not* a closure literal binds nothing and still substitutes to `mixed`; say so where you land it.

Then, in rough order:

1. **`Core\Arr::sort` is not registry-row-plus-body** — the current plan text was corrected. Its
   `{by?: callable, order?: Order, comparator?: callable, preserveKeys?: bool}` needs two things that do
   not exist: a `Core`-owned **enum** (`Order { Asc, Desc }`) the registry can state and the class graph
   can resolve `Order::Desc` against, and an **absent** option — `by`/`comparator` have no "not given"
   spelling, which is the `Const`-has-no-`null` gap in the backlog.
2. **Cheap § 1–2 rows that need nothing new**: `Str::lines`/`chunk`/`replaceAll`, `Arr::values`/`isList`/
   `contains`/`reverse`/`flatten`/`fill`. Each is one registry row plus a body plus a PHP-verified test.
3. **Settle [ADR 0009](docs/adr/0009-string-and-bytes.md) by measurement**, exactly as the loop goal's
   standing decision spells out, then `Core\Str::length`/`at`/`slice` — `crates/mwl-stdlib/src/str.rs`'s
   module doc names the two already-registered members that follow whatever it lands on.
4. Then `crates/mwl-test` and `mwl test` (loop goal, Stage 4).

## Backlog

- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed. `mwl_types::core_lib`'s module doc owns the trust rule behind
  it, and already says removing that trust is what closes it.
- **`ConstArg::Null` is the first thing `mwl_types::defaults` should grow** — `?T $x = null` is the spec's
  most common optional shape, and it now blocks `Arr::sort`'s options too. It needs `null`'s IR
  representation, which `mwl_ir::ty::Ty::Mixed`'s own doc names as still open.
- **A `Core` member cannot *return* a union or `?T`** — same `Ty::Mixed` gap;
  `mwl_stdlib::registry::CoreTy::Union`'s docs own why a parameter is fine while a return is not.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5 own both. Note `a_typed_arithmetic_loop_contains_no_call` counts
  every `call` in `Bench::sum`, so a never-taken raise block needs that guard's claim re-read first.
- **A `Core` helper cannot name the class it throws** — every `Fault::thrown` becomes `RuntimeError`
  (`mwl_runtime::Ctx::set_runtime_error_class`), so spec § 10's `LogicError` is unreachable from
  `mwl-stdlib`. `mwl_runtime::mwl_raise_new` is the shape a fix would reuse.
- **`$fn(...)`, `for`/`switch`/`match` and ADR 0043's `by`-delegation are all unlowered, and
  `crates/mwl-ir/src/lib.rs`'s module doc is the changelog CLAUDE.md forbids** — `mwl-ir`'s and
  `mwl_types::conformance`'s own docs own all four.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two state the same thing, the one CLAUDE.md names is authoritative and
the other is a bug — including this file, overwritten never appended to, capped at 80 lines by
`.claude/SESSION_PROMPT.md`. After editing any doc, run `python .claude/brief.py --check`. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. For every multi-line Rust edit: write the old and new blocks to files under `target/` with the
  Write tool, then `python .claude/splice.py <target> <old> <new>` — its own docstring says why.
- **Another agent may be editing this repo at the same time** — one overwrote this very file mid-session.
  **Stage your own paths explicitly, check `git show --stat` after committing, and re-read a shared doc
  immediately before rewriting it.**
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. One fixture:
  `.claude/leak-check.sh <paths>`, whose header says why to run it for **any** new refcount edge.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
