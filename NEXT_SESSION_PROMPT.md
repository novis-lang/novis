# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

Stage 1's **enum gate is closed**: `examples/enums.mwl` prints its three frozen lines. Four ADRs went end
to end this session, and each is owned by a doc comment rather than by a summary here:

- **ADR 0010 — an enum is an integer.** `mwl_types::enums` (new module) resolves each declaration's
  backing type and its cases' values, including the C# auto-increment rule. The backing type rides in
  `mwl_types::ty::Ty::Enum(QName, EnumBacking)` — read that variant for why it is part of the type rather
  than a side table. `mwl_ir::ty::Ty::Enum(EnumRepr)` is a representation of its own *only* because
  ADR 0035 § 4 makes an enum case always truthy where the integer under it would be falsy at `0`.
- **ADR 0007 § 2 — `as`.** `mwl_ir::lower::Lowering::convert` owns the whole table. Free rows are
  identity and the new `InstKind::Reinterpret`; total rows reuse the `Helper` conversions `.` and ADR
  0035's truthy table already had; checked rows are nine new throwing helpers in `mwl_runtime::helpers`,
  emitted through `emit_fallible` so a failed conversion travels ADR 0002's path into an ordinary `catch`.
- **ADR 0013 — object ordering.** `$a < $b` is a `Comparable::compareTo` call plus a comparison of its
  `int` against zero. Until this session it silently compared two heap pointers as integers.
- **ADR 0023 § 1 — `clone`.** `InstKind::Clone` over the new `mwl_object_clone` primitive: shallow,
  same-heap, single-level, no hook.

`ExprTypeTable` also gained `declared_ty(span)` — the checker's resolved type for a written annotation.
`mwl_ir::lower::lower_decl_type` consults it before falling back to its AST-only match, because an enum
name is the first type atom whose meaning needs the symbol table `mwl-ir` deliberately does not have.

## Next

**`examples/hooks.mwl` — the last of Stage 1.** It prints `0`/`0`/`Counter(10)`/`n=1` instead of
`6`/`20`/`Counter(10)`/`n=6`, and it needs *two* independent features. Pick one; each is a session.

1. **ADR 0014's property hooks.** `public int $doubled { get => $this->hits * 2; }` is parsed and checked
   and then ignored — a read falls through to the plain field slot, which is always `null`/`0`. The
   shape to build, mirroring what `Comparable` just did: compile each hook body as an ordinary function
   under its own label, have `mwl_types` record an `ExprInfo::Call` for a `PropertyAccess` that resolves
   to a hooked property instead of the `ExprInfo::Property` it records now, and let `mwl-ir`'s existing
   call lowering do the rest. `mwl_types::signatures` is where hook presence has to start being recorded
   (`ClassSignature` knows a property is hooked today only as an ADR 0022 exemption). `set` hooks are the
   same shape on the write side. The `PropertyObserver` half of ADR 0014 is separate and not needed by
   the fixture.
2. **By-reference parameters (`int &$slot`).** `Adder::bump($n)` is M4's `references (&$x)` bullet. This
   needs a representation decision — ADR 0007 § 1 says both sides of a reference declare the *same* type,
   so the question is what a `&T` parameter is at the ABI level, not what it means. Decide and record it
   in `mwl_runtime`'s or `mwl_ir::ty`'s own module doc, per `.claude/loop-goal.md`'s standing
   "decide and record; never BLOCKED for a design call".

After Stage 1, **Stage 2** is `examples/iterate.mwl`: ADR 0053's generators and the two iteration
interfaces, blocked first on `implements Iterable<int>` not parsing (a generic interface in an
`implements` clause).

## Backlog

Each crate's own module doc is the home for its known gaps; these are the seven worth surfacing.

- **The last conversion row: an integer *into* an enum.** ADR 0010 § 5 says it throws on a value no case
  names. `mwl_ir::lower::Lowering::convert` panics naming it. It needs the declaration's case set carried
  to the point of the check — `mwl_types::EnumTable` has it, nothing in the IR expresses it.
- **A checked conversion throws a `RuntimeError`, not an `ArithmeticError`.** `mwl_runtime::helpers`'
  `does_not_fit` owns the note: a helper failure carries only a message, so the driver promotes every one
  to the same class. ADR 0007 § 4 names the closer one.
- **Virtual dispatch through a base-typed local** — `mwl-codegen`'s known gap 1, narrowed not closed. The
  method table it needs exists (`mwl_ir::ir::Class::methods`); what is left is a compile-time slot index
  over a name lookup, which is a latency question, not a missing mechanism.
- **`new static()` calls no constructor when the statically resolved chain declares none**, even if the
  called class adds one — `mwl_ir::ir::InstKind::NewDynamic` owns the note.
- **`finally` misses two exits** — a throw out of a `catch` clause's own body, and `break`/`continue` out
  of a protected region. `mwl_ir::lower::Lowering::lower_try`'s doc comment owns both.
- **A `catch` clause inside a `namespace` block will not resolve** — `catch_clause_type` takes the written
  text. Fix as `instanceof` did: have `mwl_types` record a resolved `QName` per clause.
- **Integer `Div`/`Mod` are still refused** in `mwl-codegen` (`sdiv` traps on a zero divisor).
  `examples/core.mwl` needs `%`; ADR 0007 § 4's `int / int` union is separate and larger.
- **`crates/mwl-test` and `mwl test`** — Stage 4's two suites hold most of this loop's coverage, and
  hand-writing them as PowerShell assertions is the trap. `every_part_one_member_has_a_conformance_case`
  is cheap now: it can read the registry.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of exactly the kind CLAUDE.md
  forbids. It is the one doc in the repo genuinely owed a trim.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`.

Six tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, including inside a `python - <<'PY'` script: a
  trailing `\` silently vanishes (joining a wrapped Rust string into one long line) and `\\` becomes one
  backslash. Write the Python helper to a *file* with the Write tool and run the file.
- **`rustfmt` rewrites a `"...\n..."` literal in a test into a real multi-line string.** So a Python
  patch that matches on `\\n` inside a fixture string will stop matching after the first `cargo fmt`.
  Match on the formatted form, or use the Edit tool.
- **Write a commit message to `target/`, never the repo root** — `git add -A` picks up a root-level
  scratch file and commits it.
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` needs PowerShell**, not the Bash tool (which rewrites `/mnt/d/…`), and a **script file** —
  an inline `bash -lc "…"` mangles. Do not pipe its output through `Select-Object -First N`: that closes
  the pipe and kills the run partway. The Linux leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`.
- `cargo insta test --accept -p <crate>` is installed (note `test --accept`, not `accept -p`). Read the
  diffs first; a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
