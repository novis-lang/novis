# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

Stage 1's **exception gate is closed**: `examples/errors.mwl` prints its seven frozen lines, and
`throw`/`trace`/`uncaught.mwl` keep theirs byte for byte after migrating to spec § 10's object-shaped
`Throwable`. All four are valgrind-clean under WSL against a Linux build. `mwl_ir::Ty::Throwable` no longer
exists — an exception is an ordinary object, which closes `mwl-codegen`'s known gap 0. Its shape lives in
[`mwl_hir::errors`](crates/mwl-hir/src/errors.rs) (the tree and slot order),
`mwl_types::error_lib` (signatures), `mwl_ir::lower::Lowering::lower_try` (dispatch and `finally` policy)
and `mwl_runtime::throwable` (the pending value) — read those, not a summary here.

Stage 1 still fails on `objects.mwl`/`enums.mwl` (`static`/`self` as a declared type) and `hooks.mwl`
(ADR 0014's hooks are parsed, checked and ignored). Stages 2–4 are untouched.

## Next

**`static`/`self` as a declared type, with late static binding** — `objects.mwl` and `enums.mwl` both stop
at `mwl_ir::lower_decl_type`'s panic naming `Atom(StaticTy)`/`Atom(SelfTy)`. Scoped by the plan's **M4**
paragraph (`new static()` through two levels of inheritance returns the called class).

**Do not land only the cheap half.** Mapping all three spellings to `Ty::Object` is five lines, but
`new static()` would then allocate the *base* class and `static::tag()` would call the base's, so
`objects.mwl` would print `base`/`base is a mid` — a loud panic turned into a silent wrong answer. The work
is making the *called* class travel to the callee, which is the same mechanism virtual dispatch needs
(`mwl_runtime::ClassDesc` is the natural place to hang a vtable). Settle it under
`.claude/loop-goal.md`'s decide-and-record rule, record it in `mwl-runtime::object`'s module doc, and land
both halves together.

The alternative slice, if that one does not fit: **more `Core` rows** for `examples/core.mwl` and
`report.mwl`. A member is one `mwl_stdlib::registry::CLASSES` row, one `mwl_helper!` body and one arm in
`mwl_stdlib::symbols` — `Core\Arr::isEmpty` is the worked example (note the macro takes exactly one
function per block). Read `crates/mwl-stdlib/src/lib.rs`'s module docs first.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the six worth surfacing.

- **`finally` misses two exits** — a throw out of a `catch` clause's own body, and `break`/`continue` out
  of a protected region. `mwl_ir::lower::Lowering::lower_try`'s doc comment owns both.
- **A `catch` clause inside a `namespace` block will not resolve** — `catch_clause_type` takes the written
  text. Fix as `instanceof` did: have `mwl_types` record a resolved `QName` per clause.
- **`Throwable::$previous` can never be set** — `MethodSig` cannot model an optional parameter. See
  `mwl_types::error_lib`'s known gaps.
- **Integer `Div`/`Mod` are still refused** in `mwl-codegen` (`sdiv` traps on a zero divisor). `examples/core.mwl`
  needs `%`; ADR 0007 § 4's `int / int` union is separate and larger.
- **ADR 0014's property hooks** — `examples/hooks.mwl` prints the raw slot instead of the hook's answer.
- **`crates/mwl-test` and `mwl test`** — Stage 4's two suites hold most of this loop's coverage, and
  hand-writing them as PowerShell assertions is the trap. `every_part_one_member_has_a_conformance_case`
  is cheap now: it can read the registry.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`.

Five tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, including inside a `python - <<'PY'` script: a
  trailing `\` silently vanishes (joining a wrapped Rust string into one long line) and `\\` becomes one
  backslash. Write the Python helper to a *file* with the Write tool and run the file.
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` needs PowerShell**, not the Bash tool (which rewrites `/mnt/d/…`), and a **script file** —
  an inline `bash -lc "…"` mangles. Do not pipe its output through `Select-Object -First N`: that closes
  the pipe and kills the run partway. The Linux leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`.
- `cargo insta test --accept -p <crate>` is installed (note `test --accept`, not `accept -p`). Read the
  diffs first; a renamed test needs its old `.snap` deleted.
- `cargo test --release -p mwl-abi-probe` takes over two minutes — run it in the background.
