# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

Stage 1's **object gate is closed**: `examples/objects.mwl` prints its seven frozen lines, including late
static binding (`new static()` through two levels of inheritance, `static::tag()` from an instance method)
and ADR 0043 § 2's interface default method bodies. Arrays and exceptions were already closed.

Two mechanisms landed together, both owned by [`mwl_runtime::object`](crates/mwl-runtime/src/object.rs)'s
module doc — read that, not a summary here: the called class rides in a static method's argument slot 0
(the slot its caller already filled with `null`), and a `ClassDesc` carries a name-keyed method table
`mwl-codegen`'s `bind_method_tables` fills in after `finalize_definitions`. Exactly two call shapes
dispatch through it, both because no static answer exists — `static::m()`/`new static()`, and a call
resolving to a declaration with no *body*. Every other call is still statically resolved.

Stage 1 still fails on `examples/hooks.mwl` (ADR 0014's hooks are parsed, checked and ignored — it prints
`0`/`0`/`Counter(10)`/`n=1` instead of `6`/`20`/`Counter(10)`/`n=6`) and on `examples/enums.mwl`, which now
reaches `mwl_ir`'s panic on a `Conversion` expression. Stages 2–4 are untouched. The Linux leg was run this
session: everything Stage 1 already passes is byte-identical and valgrind-clean there, `objects.mwl`
included.

## Next

**The `as` conversion operator**, starting with the rows `examples/enums.mwl` needs: `$this->rank as int`
(an enum to its backing `int`) and `Rank::Gold as int`. `mwl_ir::lower`'s expression arm panics naming
`Conversion { .. }`; ADR 0007's conversion table is the scope, and M4's acceptance wants *every* row of it
both succeeding and throwing. Doing the enum rows first is what unblocks Stage 1; `int`↔`string`↔`float`
is the natural second sweep and the one `Core\Str`'s conformance cases will lean on.

`enums.mwl` needs three more things after that, each small on its own: `Comparable` reaching `$ana > $bo`
(ADR 0013), `clone` (ADR 0023 § 1's shallow same-heap copy, which `mwl_runtime::MwlObj` already expresses),
and enum declarations reaching `mwl_ir::lower_file` at all — it still skips `StmtKind::EnumDecl` the way it
skipped interfaces until this session.

The alternative slice, if that one does not fit: **ADR 0014's property hooks**, the other half of Stage 1
and independent of the conversion work. `examples/hooks.mwl` is the fixture, ADR 0014 the rule.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the six worth surfacing.

- **Virtual dispatch through a base-typed local** — `mwl-codegen`'s known gap 1, narrowed not closed. The
  method table it needs exists (`mwl_ir::ir::Class::methods`); what is left is a compile-time slot index
  over a name lookup, which is a latency question, not a missing mechanism.
- **`new static()` calls no constructor when the statically resolved chain declares none**, even if the
  called class adds one — `mwl_ir::ir::InstKind::NewDynamic` owns the note.
- **`finally` misses two exits** — a throw out of a `catch` clause's own body, and `break`/`continue` out
  of a protected region. `mwl_ir::lower::Lowering::lower_try`'s doc comment owns both.
- **A `catch` clause inside a `namespace` block will not resolve** — `catch_clause_type` takes the written
  text. Fix as `instanceof` did: have `mwl_types` record a resolved `QName` per clause.
- **Integer `Div`/`Mod` are still refused** in `mwl-codegen` (`sdiv` traps on a zero divisor). `examples/core.mwl`
  needs `%`; ADR 0007 § 4's `int / int` union is separate and larger.
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
