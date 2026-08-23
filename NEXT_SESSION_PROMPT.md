# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — authoritative
for the acceptance list and for the ten standing decisions already settled with the user; do not re-open
any of them. The plan's status block says what is on disk and what is open.

Stages 1 and 2 are green on both legs, byte for byte, with `valgrind --leak-check=full` clean. **ADR 0053
is done in full. ADR 0031 §§ 1–2 landed this session** — closures are checked, lowered, compiled and
callable from native `Core` code, with `Core\Arr::filter` as the first member that does. That ADR's own
*Verification* list says what it covers and what is deliberately still open inside it; the shapes are
owned by `mwl_types::expr::check_fn_literal`, `mwl_ir::lower::lower_closure` and `mwl_runtime::closure`
— read those three doc comments rather than looking for a summary here.

Two documentation-only decisions landed alongside, both M6-scope and neither touching the loop:
[ADR 0064](docs/adr/0064-configuration-file-format.md) (TOML in `mwl.toml`, `Core\Config::set`) and
[ADR 0065](docs/adr/0065-third-party-attribution-and-mwl-info.md) (attribution and `mwl info`). Nothing
on disk parses `mwl.toml` yet. **Nothing is blocked**; `examples/core.mwl` now stops at its *first* line,
on an unregistered `Core\Arr::range`.

## Next

**Stage 3 — `Core` §§ 1–12**, per [docs/spec/01-core-library.md](docs/spec/01-core-library.md), which is
authoritative for every signature, shaped by [ADR 0063](docs/adr/0063-core-api-conventions.md). Three
session-sized slices stand between here and `examples/core.mwl`'s six frozen output lines. Take them in
this order — each is smaller than the next and unblocks it:

1. **Integer `/` and `%`.** `mwl-codegen` refuses `Div`/`Mod` because `sdiv` traps the process on a zero
   divisor, which is a request-isolation failure. The throw path exists, so this is a checked divisor plus
   a `Terminator::Throw`, against ADR 0007 § 4's arithmetic table. `core.mwl` line 3 needs `%`.
2. **An options-shape argument and an optional parameter.** `Core\Arr::range(int, int, {step?: int})` and
   `sort(array<T>, {by?: callable, …})` need `mwl_stdlib::registry::CoreTy` to express a shape and the
   arity check to accept a missing trailing argument — `mwl-stdlib`'s own known gap 3. ADR 0063 R2 makes
   this the shape of *every* optioned member, so do it properly once. `mwl_types::ty::Ty::Shape` and ADR
   0036 § 3's width subtyping already exist on the checker side.
3. **`Core\Str` rows** — `upper`, `lower`, `join`, `split`, `replace`, `padStart`, `padEnd`, `upperFirst`,
   `length`. **Settle [ADR 0009](docs/adr/0009-string-and-bytes.md) by measurement first**, exactly as the
   loop goal's standing decision spells out, *before* writing conformance cases. Only `length`/`at`/`slice`
   depend on it, so the other rows can land ahead of it.

Then `Core\Arr::map`/`sort`, which the closure work already unblocked — note `map`'s `array<U>` return
substitutes an unbound `U` to `mixed` today, which `Core\Str::join` will refuse, so `map` needs that
decided before it is useful. Then `crates/mwl-test` and `mwl test` (loop goal, Stage 4).

## Backlog

- **`$fn(...)` has no lowering, and ADR 0031 § 3's self-name is parsed and ignored** — `mwl-ir`'s known
  gaps own both. Direct invocation needs a decision first: `callable` is opaque, so `$f(3)` has no
  argument types to check and no result type but `mixed`.
- **`for`/`switch`/`match` are still unlowered** — `mwl-ir`'s module doc. `Terminator::Switch` was built
  general precisely so `switch` reaches for it.
- **ADR 0043's `by`-delegation is unimplemented**, and `mwl_types::conformance` exempts any class using
  one *whole* because of that — its doc comment owns why the two close together.
- **ADR 0014's `PropertyObserver` half is untouched** — §§ 2–3; nothing on the acceptance path needs it.
- **A helper that fails leaks its borrowed temporaries** — `Lowering::release_call_temporaries`, the same
  owned-temporaries stack `Lowering::landing_block`'s gap already needs.
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
- **Another agent may be editing this repo at the same time.** `git status` grew files this session that
  no commit here touched. **Stage your own paths explicitly and check `git show --stat` after committing**
  — a `git add -A` swept an unrelated untracked file into one commit and had to be amended back out.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. The full leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. For a narrower check write a one-off script plus
  fixture under `target/` — that caught this session's leak; repeat it for **any** hand-written refcount.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
