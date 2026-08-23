# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

**Stages 1 and 2 are green on both legs.** Every command, byte for byte, on Windows and under WSL against
a Linux build, with `valgrind --leak-check=full` clean on all eleven runnable fixtures —
`examples/iterate.mwl` prints `generator=15` then `iterable=10`. **ADR 0053 is done in full**; its own
*Verification* section says what that covers and the two small things still open inside it. The shape is
owned by doc comments rather than summarised here:

- **`mwl_ir::lower::lower_generator`** owns § 4's transform: one source declaration becomes a factory
  (which runs no user code — it parks the receiver and every argument in a state object and returns it),
  an `advance()` holding the original body, a `current()`, and a synthesized state class. A `yield` spills
  every `Env` binding into that class and leaves the frame with `true`; the resume block reads them all
  back. So a value never lives *across* a suspension in SSA form, and the resume block is an ordinary
  block the enclosing `while`/`if`/`try` carries on from.
- **`Lowering::seed_generator_loop_carried`** is the one place a generator changes the surrounding
  lowering: the entry switch enters a resume block that may sit *inside* a loop body, so a pre-loop value
  no longer dominates the header — every binding gets a header phi in a generator, and nothing else.
- **`mwl_ir::ir::Terminator::Switch`** is the N-way terminator the crate docs predicted resumption and
  `switch` would share. Built general (unsorted, non-dense, one `EdgeId` per arm). `mwl-codegen` lowers it
  to a compare chain — its known gap 6.
- **`Lowering::lower_foreach_cursor`** is § 3's other two subjects. `mwl_types::ExprTypeTable::foreach_drive`
  is how `mwl-ir` learns which of the three shapes a subject is (it cannot re-derive it — reaching
  `Iterable` through a base class is a `ClassGraph` walk).
- **`mwl_types::conformance`** holds a class to every interface method it inherits without a body. Its
  docs own the three exemptions. § 1's deliberately bodiless `advance`/`current` are what asked for it.
- **`mwl_types::expr::class_satisfied`** is MWL's one nominal subtyping rule, added because without it no
  `iterate()` can return a concrete cursor class: a class satisfies anything it reaches through
  `extends`/`implements`, invariantly in a generic target's arguments.

## Next

**Stage 3 — `Core` §§ 1–12, and the two language features `examples/core.mwl` calls them through.**
This is the bulk of M4S and will take several sessions; the loop goal's *Standing decisions* already
settle every design question it raises (`Core` is native Rust in `crates/mwl-stdlib`, reached through a
signature table the checker knows and a helper symbol codegen emits; dependencies are picked against
ADR 0051 § 4 and recorded in the module's own doc comment; ADR 0009 is settled by measurement *before*
`Core\Str`'s conformance cases are written).

Read `examples/core.mwl` and `examples/report.mwl` — they are the acceptance, and between them they name
exactly what the first session needs. Two of those needs are *language*, not library, and are the natural
place to start because nothing in `Core` can be exercised without them:

1. **Closures.** `Core\Arr::filter($nums, fn(int $n) => $n % 2 === 0)` needs ADR 0031's `fn` literal to
   parse (it does), check and lower. `mwl_ir::lower` has no `ExprKind::Fn` arm at all.
2. **Shape literals in argument position.** `Core\Arr::sort($words, {by: fn(string $w) => ...})` is ADR
   0063's trailing options shape, checked structurally per ADR 0036 § 3.
3. **Integer `%`** — `mwl-codegen` still refuses `Div`/`Mod` because `sdiv` traps the whole process on a
   zero divisor, which is a request-isolation failure. The throw path exists now, so this is a checked
   divisor plus a `Terminator::Throw`. `examples/core.mwl` uses `%`, so it is on the path.

Pick the slice that fits one session. Closures first is the obvious order — everything else in
`core.mwl` is a `Core` row, and a row with no way to pass it a callback cannot be conformance-tested.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the ones worth surfacing.

- **`Iterator::current()` does not throw before the first `advance()` or after one returns `false`** —
  ADR 0053 § 1 says it must. `mwl_ir::lower::lower_generator_current`'s doc owns the gap; a `foreach`, the
  only thing driving a cursor today, never calls it at either point.
- **A generator with a `&$x` parameter, or a `Ty::Ref` binding live across a `yield`, panics naming
  itself** — the cell it addresses is the caller's, and the caller is gone by the time it resumes.
- **`static::`/`new static()` inside a generator body** reaches `Lowering::lsb`'s panic: `Lowering::this`
  is `None` there, because `$this` is an ordinary reloaded local rather than a value from a block that
  dominates every resume point.
- **A `foreach` key binding over an array is unchecked**, because `array<T>` records no key type at all —
  `mwl_types::expr::check_foreach_key`'s doc comment owns the reasoning and names `array<K, V>` as the
  prerequisite. Not a bug; a bounded decision deferred.
- **A name in type position is lowered twice**, so `E0441`/`E0442`/`E0303` on a parameter type each print
  twice. Pre-existing and cosmetic, but loud.
- **A by-reference call is lowered in two positions only** — `Lowering::pending_refs` owns why, and the
  fix is threading `&mut Env` through `lower_expr`, the same widening `landing_block`'s in-flight-
  temporary gap needs.
- **A callee that throws never reaches its by-reference copy-back**, leaking the staging retain — never
  dangling. `mwl_ir::Ty::Ref`'s *Known gap* names the exact fix.
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
- **`for`/`switch`/`match` are still unlowered.** Every shape they need now exists, `Terminator::Switch`
  included — it was built general rather than resumption-specific precisely so `switch` reaches for it.
- **ADR 0043's `by`-delegation** is unimplemented, and `mwl_types::conformance` exempts any class using
  one *whole* because of that — closing the delegation gap and narrowing that exemption go together.
- **`crates/mwl-test` and `mwl test`** — Stage 4's two suites hold most of this loop's coverage.
  `every_part_one_member_has_a_conformance_case` is cheap now: it can read the registry.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of exactly the kind CLAUDE.md
  forbids. It is the one doc in the repo genuinely owed a trim.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`.

Seven tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, including inside a `python - <<'PY'` script: a
  trailing `\` silently vanishes (joining a wrapped Rust string into one long line) and `\\` becomes one
  backslash. This bit again this session, on a `\`-continued Rust string inside a diagnostic's help text.
  Write the Python helper to a *file* with the Write tool and run the file, or use the Edit tool.
- **`gen` is a reserved keyword in Rust 2024.** A field or local named `gen` is a parse error with a
  confusing message; `generator` is what `mwl_ir::lower` uses.
- **`rustfmt` rewrites a `"...\n..."` literal in a test into a real multi-line string.** So a Python
  patch that matches on `\\n` inside a fixture string will stop matching after the first `cargo fmt`.
  Match on the formatted form, or use the Edit tool.
- **Write a commit message to `target/`, never the repo root** — `git add -A` picks up a root-level
  scratch file and commits it.
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` needs PowerShell**, not the Bash tool (which rewrites `/mnt/d/…`), and a **script file** —
  an inline `bash -lc "…"` mangles. Do not pipe its output through `Select-Object -First N`: that closes
  the pipe and kills the run partway. The Linux leg is
  `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`, and it is worth running: it is the only thing
  that catches a leak in a hand-written refcount protocol.
- `cargo insta test --accept -p <crate>` is installed (note `test --accept`, not `accept -p`). Read the
  diffs first; a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
