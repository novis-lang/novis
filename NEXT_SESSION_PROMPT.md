# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — authoritative
for the acceptance list and for the ten standing decisions already settled with the user; do not re-open
any of them. The plan's status block says what is on disk and what is open.

Stages 1 and 2 are green on both legs, byte for byte. **An optional parameter now works end to end** —
user-declared and `Core` alike, with the constant materialized at the call site (`mwl_types::defaults`
owns why) — and **`Core\Str`'s twelve granularity-free members are registered and implemented**,
`Core\Str::join` being the first `Core` member with a default. Both new refcount edges are
`valgrind --leak-check=full` clean under WSL. **Nothing is blocked**; `examples/core.mwl` still stops at
its *first* line, on an unregistered `Core\Arr::range`.

## Next

**The options shape** — [ADR 0063](docs/adr/0063-core-api-conventions.md) R2's trailing bag, and the
largest single thing keeping the §§ 1–2 roster unregistered. **The design is settled and written down:
read `crates/mwl-stdlib/src/registry.rs`'s module docs, § *The options bag, and the shape it will take*,
and implement that rather than re-deriving the fork.** In one line: a compiler-owned `Ty::Options` beside
ADR 0036's `Ty::Shape`, always last and always optional, flattened into one ABI argument per option at
`mwl_ir::lower::lower_call_args`. `Core\Arr::range(int, int, {step?: int})` is the first consumer and it
unblocks `examples/core.mwl`'s first line — PHP 8.5 refuses a zero or negative `$step`, verified.

Then, all now pure registry-row-plus-body work, in roughly this order:

1. **`Core\Arr::map`/`sort`**, which the closure work already unblocked — note `map`'s `array<U>` return
   substitutes an unbound `U` to `mixed` today, which `Core\Str::join` now *refuses*, so `map` needs that
   decided before it is useful. `examples/core.mwl` line 8 is the case, and `mwl check` shows it.
2. **`Core\Str::replace`**, an options-bag member, once the bag exists.
3. **Settle [ADR 0009](docs/adr/0009-string-and-bytes.md) by measurement**, exactly as the loop goal's
   standing decision spells out, then `Core\Str::length`/`at`/`slice`. `crates/mwl-stdlib/src/str.rs`'s
   module doc names the two already-registered members that follow whatever it lands on.
4. Then `crates/mwl-test` and `mwl test` (loop goal, Stage 4).

## Backlog

- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed. `mwl_types::core_lib`'s module doc owns the trust rule behind
  it, and already says removing that trust is what closes it.
- **`ConstArg::Null` is the first thing `mwl_types::defaults` should grow** — `?T $x = null` is the spec's
  most common optional shape and is refused today. It needs `null`'s IR representation, which
  `mwl_ir::ty::Ty::Mixed`'s own doc names as still open.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5 own both. Note `a_typed_arithmetic_loop_contains_no_call` counts
  every `call` in `Bench::sum`, so a never-taken raise block needs that guard's claim re-read first.
- **`decimal`: M2 must place a literal at `as T`, and `19.99m` must not lex** —
  [ADR 0054](docs/adr/0054-decimal-scalar-type.md) § 2 dropped the `m` suffix and states what makes
  `19.99 as decimal` safe; `float → decimal` would keep only ~17 of the 29 digits M8 needs.
- **A `Core` helper cannot name the class it throws** — every `Fault::thrown` becomes `RuntimeError`
  (`mwl_runtime::Ctx::set_runtime_error_class`), so spec § 10's `LogicError` is unreachable from
  `mwl-stdlib`. `mwl_runtime::mwl_raise_new` is the shape a fix would reuse.
- **`$fn(...)` has no lowering, `for`/`switch`/`match` are unlowered, ADR 0043's `by`-delegation is
  unimplemented, and `crates/mwl-ir/src/lib.rs`'s module doc is the slice-by-slice changelog CLAUDE.md
  forbids** — `mwl-ir`'s and `mwl_types::conformance`'s own docs own all four.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two state the same thing, the one CLAUDE.md names is authoritative and
the other is a bug — including this file, overwritten never appended to, capped at 80 lines by
`.claude/SESSION_PROMPT.md`. After editing any doc, run `python .claude/brief.py --check`. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, `python - <<'PY'` included: a trailing `\` vanishes
  and `\\` becomes one. So does an escaped `\"` inside a Rust test fixture. Write new code to a file under
  `target/` with the Write tool and splice it in with a script that matches only on plain text; edit an
  existing string with the Edit tool.
- **Another agent may be editing this repo at the same time** — one overwrote this very file mid-session.
  **Stage your own paths explicitly, check `git show --stat` after committing, and re-read a shared doc
  immediately before rewriting it.**
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and its stdout
  and stderr interleave — redirect to a file inside the script and `cat` it. The full leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. For a narrower check write a one-off script plus
  fixture under `target/` — `target/defaults-leak.sh` is this session's, and is the template; repeat it
  for **any** new error edge or hand-written refcount.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
