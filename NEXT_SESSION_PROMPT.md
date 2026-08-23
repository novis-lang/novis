# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

**Stage 1 is green in full.** All thirteen of its commands pass, byte for byte against the frozen
output — `hooks.mwl` now prints `n=6`. By-reference parameters (`int &$slot`) landed this session; the
shape is owned by the doc comments rather than summarised here:

- **`mwl_ir::Ty::Ref` owns the representation decision** — a *caller-staged one-cell slot*, the second
  of the two models the last session weighed. The caller allocates one `Value`-sized stack slot, copies
  the holder's current value in, passes its address, and copies back after the call. True PHP aliasing
  was rejected on cost, and that doc comment says why in full.
- Three primitive instructions carry it: `InstKind::RefSlot`/`RefLoad`/`RefStore`, each a pure
  address/load/store in `mwl-codegen`. The refcount policy is in lowering, the way `bind_local`'s
  already is — a staging `Retain`, a copy-back `Release`, so **the slot owns exactly one reference at
  every point**.
- `mwl_types::expr::check_by_ref_arg` adds the two obligations that keep it sound: a writable place
  (`E0439`) and an *exactly* matching type (`E0440`). The second is load-bearing: `is_assignable` lets a
  plain `string` satisfy a `tainted string` parameter, and accepting that by reference would launder
  taint through an argument list.
- `Lowering::collect_reassigned_locals` gained a by-reference half. A `&$x` argument re-points its
  holder just as an assignment does, but nothing in the call's *syntax* says so — the `&` is on the
  callee's declaration — so a loop body's header phi has to be read off the resolved signature.

## Next

**Stage 2 — `examples/iterate.mwl`, which is ADR 0053 end to end.** It is the whole of Stage 2, and it
needs three things, in this order. Nothing here is a design call: ADR 0053 settles all of it.

1. **`Iterable` and `Iterator` do not exist as declarations.** `mwl check` reports `E0303: `Iterable`
   is not declared` before it reaches the parse error. They are reserved global interfaces, exactly the
   way `Comparable`/`Stringable` already are — see `mwl_hir`'s seeding of those (`mwl_hir::errors`
   seeds the exception tree the same way) and `mwl_types::error_lib`/`core_lib` for how a native
   declaration gets a signature with no source text. ADR 0053 § 1 fixes both shapes:
   `Iterable<T>::iterate(): Iterator<T>` and `Iterator<T>`'s own member set. Do that first — it is
   independent of the parser work and unblocks the type side of everything below.
2. **A generic interface in an `implements` clause does not parse.** `implements Iterable<int>` is
   `.claude/loop-goal.md`'s named gap and ADR 0053 § 2's one narrow extension to the standing decision
   that user-declared generics stay parked. The loop goal's *Standing decisions* is explicit about the
   boundary: user code gets exactly two things, implementing a compiler-owned generic interface at a
   concrete type, and the explicit call-site type argument. `mwl_types::generics` already has
   substitution and call-site inference, so this is a parse + resolve slice, not a solver.
3. **Generators.** `yield` in `Counter::upTo`, lowered to a state machine per ADR 0053 § 3 — never a
   coroutine, so every compile target keeps them. `mwl-ir` has no `switch` either, and the loop goal
   notes the two pair naturally: resumption wants the same N-way terminator `switch` does. Expect that
   to be the bulk of the session, and expect it to want its own representation paragraph in
   `mwl_ir::lower`'s or `mwl_ir::ir`'s module doc, not a numbered ADR.

`foreach` already lowers over an `array<T>` subject (`Lowering::lower_foreach` owns the whole policy);
what it refuses is an `Iterable`/`Iterator` subject, which is item 1's consumer.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the ones worth surfacing.

- **A by-reference call is lowered in two positions only** — a bare expression statement, or a plain
  assignment's right-hand side. `Lowering::pending_refs` owns why (the copy-back needs an `&mut Env`,
  which only the statement level holds) and `lower_stmts` asserts rather than silently dropping the
  write-back. The fix is threading `&mut Env` through `lower_expr`, which is the same widening
  `landing_block`'s in-flight-temporary gap needs.
- **A callee that throws never reaches its by-reference copy-back**, leaking the staging retain — never
  dangling, which is what the unconditional retain buys. `mwl_ir::Ty::Ref`'s *Known gap* section names
  the exact fix: emit the same `RefLoad` and write-back into `landing_block`'s block ahead of its
  release sweep, and update the `TryFrame` edge's captured `Env`.
- **`is_assignable` models no class subtyping at all** — `Dog` into an `Animal` parameter is
  `E_TYPE_MISMATCH`. Found while writing this session's `E0440` test, which had to be rewritten around
  it. Not on the acceptance path (no Stage 1–3 fixture passes a subclass into a base-typed position),
  but it is a real hole in `mwl_types::expr::is_assignable` and every PHP program in existence relies
  on it.
- **An array-element write through a hooked property** (`$obj->hooked[0] = v`) panics naming itself —
  `mwl_ir::lower::Lowering::write_back_array`. No PHP-compatible rule for writing the copy-on-write
  separation back through a `set` hook exists yet.
- **ADR 0014's `PropertyObserver` half** is untouched — § 2/§ 3's declared interface, called after the
  hook or storage settles. Independent of § 1, and not needed by any fixture.
- **A hook on a `static` or `readonly` property is not refused.** Neither combination means anything;
  `mwl_types::check::check_property_hooks` is where the diagnostic would go.
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
