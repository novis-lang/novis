# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

**Stage 1 is green in full** and stayed green through this session. **Stage 2's type side is now done
too**: `examples/iterate.mwl` passes `mwl check` with no errors, and everything left in it is lowering.
The shape is owned by doc comments rather than summarised here:

- **`mwl_hir::interfaces::RESERVED`** is the roster of every global interface the compiler declares,
  with each one's type parameters — `Comparable`, `Stringable`, `Iterable<T>`, `Iterator<T>`. Same
  seeded-from-data footing `mwl_hir::errors` gives the exception tree, and
  `QName::is_reserved_global_interface` reads it rather than matching names inline.
- **`mwl_types::iter_lib`** turns the two generic entries into ADR 0053 § 1's member set. Every member
  is bodiless *on purpose* — that module's docs own why: a call resolving to a bodiless declaration has
  no compiled function to name, so it dispatches on the receiver's runtime class, which is exactly what
  driving a cursor of unknown concrete class needs.
- **`mwl_types::ty::Ty::Class` carries type arguments.** Interning is structural, so `Iterator<int>` and
  `Iterator<string>` are two `TypeId`s. They are **erased at the `mwl-ir` boundary** — that doc comment
  states it, and `lower_checked_ty` maps every class to one pointer type.
- **`crate::generics` now has two binding sites, not one.** A `Core` member takes its variable from an
  *argument*; an iteration interface takes it from the *receiver*
  (`mwl_types::expr::substitute_receiver_args`), since `$cursor->current()` has no argument list to read
  `T` out of. Both end at the same `substitute`, and the "a type variable never survives a call site"
  property is unchanged.
- **`mwl_types::expr::foreach_source`** is ADR 0053 § 3's three-shapes rule. A class's element type comes
  from `ClassSignature::implements`, walked through `extends`/`implements` by
  `signatures::resolve_iteration_element`. `E0443` refuses a fourth shape, `E0444` refuses a key binding
  over a cursor. `E0441`/`E0442` refuse a type-argument list on a name that takes none, and a wrong count
  on one that does.

## Next

**Stage 2's lowering half — generators, and `foreach` over a cursor.** ADR 0053 § 4 settles the design;
none of this is a design call. Expect this to be the bulk of the session.

1. **`yield` panics in `mwl_ir::lower`** — `lower_stmts` refuses `StmtKind::Yield` as "not a plain
   assignment or bare call". § 4 fixes the transform: a function whose body contains `yield` is a
   generator, calling it runs no user code but allocates and returns a state object implementing
   `Iterator<T>`, and the body is split at each `yield` into resumption states with every local live
   across one stored in the state object rather than on a stack. **Not** the coroutine substrate — that
   is the whole point of § 4, and ADR 0025's cross-target claim rests on it.
2. **`switch` is still absent from `mwl-ir`**, and the loop goal notes the two pair naturally: resumption
   wants the same N-way terminator `switch` does. Build the terminator once.
3. **`Lowering::lower_foreach` asserts on a non-`array<T>` subject** — its own `# Panics` doc names the
   case. The checker now hands it a fully resolved element type either way, so what is left is emitting
   `iterate()` once for an `Iterable` and then the `advance()`/`current()` drive loop, both as ordinary
   instance calls on a bodiless declaration.

Expect this to want its own representation paragraph in `mwl_ir::lower`'s or `mwl_ir::ir`'s module doc,
not a numbered ADR — the loop goal's *Standing decisions* pre-authorizes exactly that.

Also worth doing while you are in ADR 0053: **`yield`'s own diagnostics** are still missing on the
checker side (a generator whose declared return type is not `Iterator<T>`; a `yield` whose operand does
not satisfy `T`; a `yield` outside a generator body). ADR 0053's *Verification* M2 bullet names them as
the one thing still open there.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the ones worth surfacing.

- **Interface conformance is unchecked, for every interface in the language.** A class claiming
  `implements Iterator<int>` need not declare `advance`/`current` — `mwl_types::iter_lib`'s *Known gap*
  owns this. It was harmless while `Comparable` was the only case; it is not harmless for a cursor, where
  a missing member ends in a dispatch to nothing rather than in a call the author wrote. The fix is one
  check over `ClassSignature::implements`, which is why that field records resolved arguments.
- **A `foreach` key binding over an array is unchecked**, because `array<T>` records no key type at all —
  `mwl_types::expr::check_foreach_key`'s doc comment owns the reasoning and names `array<K, V>` as the
  prerequisite. Not a bug; a bounded decision deferred.
- **A name in type position is lowered twice**, so `E0441`/`E0442`/`E0303` on a parameter type each print
  twice. Pre-existing (an undeclared class already did this) and cosmetic, but loud.
- **A by-reference call is lowered in two positions only** — `Lowering::pending_refs` owns why, and the
  fix is threading `&mut Env` through `lower_expr`, the same widening `landing_block`'s in-flight-
  temporary gap needs.
- **A callee that throws never reaches its by-reference copy-back**, leaking the staging retain — never
  dangling. `mwl_ir::Ty::Ref`'s *Known gap* names the exact fix.
- **`is_assignable` models no class subtyping at all** — `Dog` into an `Animal` parameter is
  `E_TYPE_MISMATCH`. Not on the acceptance path, but every PHP program in existence relies on it.
- **An array-element write through a hooked property** (`$obj->hooked[0] = v`) panics naming itself —
  `mwl_ir::lower::Lowering::write_back_array`.
- **ADR 0014's `PropertyObserver` half** is untouched — § 2/§ 3's declared interface, called after the
  hook or storage settles. Independent of § 1, and not needed by any fixture.
- **A hook on a `static` or `readonly` property is not refused** —
  `mwl_types::check::check_property_hooks` is where the diagnostic would go.
- **The last conversion row: an integer *into* an enum.** ADR 0010 § 5 says it throws on a value no case
  names; `mwl_ir::lower::Lowering::convert` panics naming it. `mwl_types::EnumTable` has the case set,
  nothing in the IR expresses it.
- **A checked conversion throws a `RuntimeError`, not an `ArithmeticError`** — `mwl_runtime::helpers`'
  `does_not_fit` owns the note; ADR 0007 § 4 names the closer one.
- **Virtual dispatch through a base-typed local** — `mwl-codegen`'s known gap 1, narrowed not closed.
- **`new static()` calls no constructor when the statically resolved chain declares none** —
  `mwl_ir::ir::InstKind::NewDynamic` owns the note.
- **`finally` misses two exits** — a throw out of a `catch` clause's own body, and `break`/`continue` out
  of a protected region. `mwl_ir::lower::Lowering::lower_try`'s doc comment owns both.
- **A `catch` clause inside a `namespace` block will not resolve** — `catch_clause_type` takes the
  written text. Fix as `instanceof` did: have `mwl_types` record a resolved `QName` per clause.
- **Integer `Div`/`Mod` are still refused** in `mwl-codegen` (`sdiv` traps on a zero divisor).
  `examples/core.mwl` needs `%`; ADR 0007 § 4's `int / int` union is separate and larger.
- **`crates/mwl-test` and `mwl test`** — Stage 4's two suites hold most of this loop's coverage.
  `every_part_one_member_has_a_conformance_case` is cheap now: it can read the registry.
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
  backslash. This bit again this session on a `\`-continued Rust string literal. Write the Python helper
  to a *file* with the Write tool and run the file, or use the Edit tool.
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
