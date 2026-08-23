# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — authoritative
for the acceptance list and for the ten standing decisions already settled with the user; do not re-open
any of them. The plan's status block says what is on disk and what is open.

Stages 1 and 2 are green on both legs, byte for byte, with `valgrind --leak-check=full` clean. ADR 0053 is
done in full; ADR 0031 §§ 1–2 and ADR 0065 landed end to end. **Integer `%` landed this session** — a zero
divisor throws spec § 10's `ArithmeticError` instead of trapping the process, raised inline by
`mwl_runtime::mwl_raise_new` off `Inst::on_error`'s edge; `Emitter::emit_int_mod` and
`mwl_ir::ir::Inst::on_error` own the shape, and PHP 8.5 agrees on every sign case. **Nothing is blocked**;
`examples/core.mwl` still stops at its *first* line, on an unregistered `Core\Arr::range`.

## Next

**Stage 3 — `Core` §§ 1–12**, per [docs/spec/01-core-library.md](docs/spec/01-core-library.md), which is
authoritative for every signature, shaped by [ADR 0063](docs/adr/0063-core-api-conventions.md). Two
session-sized slices stand between here and `examples/core.mwl`'s six frozen output lines:

1. **An options-shape argument and an optional parameter.** Bigger than last session's estimate:
   `MethodSig` has no notion of an optional parameter *at all*, so a user-declared `int $b = 3` is refused
   by the same arity check — `check_args_typed` in `mwl_types::expr`. Doing it properly once means a
   required-count on `MethodSig`, a default materialized at the call site in `mwl_ir::lower`, and
   `mwl_stdlib::registry::CoreTy` growing a shape variant (its own known gap 3). ADR 0063 R2 makes the
   trailing options bag the shape of *every* optioned member, and `mwl_types::ty::Ty::Shape` plus ADR
   0036 § 3's width subtyping already exist on the checker side. `Core\Arr::range(int, int, {step?: int})`
   is the first consumer.
2. **`Core\Str` rows** — `upper`, `lower`, `join`, `split`, `replace`, `padStart`, `padEnd`, `upperFirst`,
   `length`. **Settle [ADR 0009](docs/adr/0009-string-and-bytes.md) by measurement first**, exactly as the
   loop goal's standing decision spells out, *before* writing conformance cases. Only `length`/`at`/`slice`
   depend on it, so the other rows can land ahead of it.

Then `Core\Arr::map`/`sort`, which the closure work already unblocked — note `map`'s `array<U>` return
substitutes an unbound `U` to `mixed` today, which `Core\Str::join` will refuse, so `map` needs that
decided before it is useful. Then `crates/mwl-test` and `mwl test` (loop goal, Stage 4).

## Backlog

- **Integer `+`/`-`/`*` wrap instead of throwing on overflow** — `mwl-codegen`'s known gap 8, which owns
  why the mechanism now exists and what is left. Note `a_typed_arithmetic_loop_contains_no_call` counts
  every `call` in `Bench::sum`, so a never-taken raise block needs that guard's claim re-read first.
- **Integer `/` is refused two phases deep** — `mwl-codegen`'s known gap 5: `int|float` has no IR
  representation, *and* `mwl_types` does not widen that union to `float` at a binding.
- **`$fn(...)` has no lowering, and ADR 0031 § 3's self-name is parsed and ignored** — `mwl-ir`'s known
  gaps own both. Direct invocation needs a decision: `callable` is opaque, so `$f(3)` has no argument
  types to check and no result type but `mixed`.
- **`for`/`switch`/`match` are still unlowered** — `mwl-ir`'s module doc. `Terminator::Switch` was built
  general precisely so `switch` reaches for it.
- **ADR 0043's `by`-delegation is unimplemented**, and `mwl_types::conformance` exempts any class using
  one *whole* because of that — its doc comment owns why the two close together.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** CLAUDE.md forbids. Every other
  open item is a *Known gap* section in the crate that owns it; look there, not here.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two state the same thing, the one CLAUDE.md names is authoritative and
the other is a bug — including this file, overwritten never appended to, capped at 80 lines by
`.claude/SESSION_PROMPT.md`. After editing any doc, run `python .claude/brief.py --check`. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, `python - <<'PY'` included: a trailing `\` vanishes
  and `\\` becomes one. So does an escaped `\"` inside a Rust test fixture. Write new code to a file under
  `target/` with the Write tool and splice it in with a script that matches only on plain text; edit an
  existing string with the Edit tool.
- **Another agent may be editing this repo at the same time.** **Stage your own paths explicitly and check
  `git show --stat` after committing** — a `git add -A` once swept an unrelated untracked file into a
  commit and had to be amended back out.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and its stdout
  and stderr interleave — redirect to a file inside the script and `cat` it. The full leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. For a narrower check write a one-off script plus
  fixture under `target/` — that is how this session's `%` error path was leak-checked; repeat it for
  **any** new error edge or hand-written refcount.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
