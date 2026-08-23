# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

**Stages 1 and 2 are green on both legs**, byte for byte, on Windows and under WSL against a Linux build,
with `valgrind --leak-check=full` clean. **ADR 0053 is done in full**, and **ADR 0031 §§ 1–2 landed**:
closures are checked, lowered, compiled and callable from native `Core` code. Each shape is
owned by a doc comment rather than summarised here:

- **`mwl_types::expr::check_fn_literal`** checks a closure body in a scope of its own and offers every
  outer binding to it as a *capture*, recorded at **`mwl_types::locals::LocalScope::declared_ty`** — the
  one lookup a read or a write already goes through, so the set is ADR 0031 § 2's "exactly the outer
  variables its body reads" and cannot drift from what the checker counts as a read. `$this` is in that
  set like any other name, which is ADR 0008 § 4 satisfied with no code of its own.
- **`mwl_ir::lower::lower_closure`** makes a closure an *object* of a synthesized class: one field per
  capture, one `invoke` method, no supertypes. That reuses refcounting, field slots, class descriptors and
  the indirect call the runtime already has, instead of adding a second refcounted heap shape. Its doc
  comment states the cost (one allocation per literal evaluation, even capturing nothing).
- **`mwl_runtime::call_closure`** is how native code reaches one, and the one place the borrow/own
  mismatch between a helper and a compiled method is reconciled. A closure carries its own arity in field
  slot 0 (`FN_ARITY` / `CLOSURE_ARITY_SLOT`) because spec § 2 hands every callback `($value, $key)` and
  lets it declare fewer parameters.
- **`mwl_runtime::Fault::Pending`** returns a callee's status unchanged, so an exception a closure raised
  is not replaced by a message from the helper frame it passed through.
- **`mwl_ir::lower::LoweredArgs`** — the leak fix valgrind found. A `Borrowed` (helper/`Core`) call now
  releases each argument the argument *expression itself built*, which is the case with no other owner.
  Pre-existing, but only reachable once a `Core` member could take a heap argument the caller just made.

**Two documentation-only decisions landed alongside, neither touching the loop.** Both are M6-scope, so
they cost a doc pass now and would cost a breaking change later:

- **[ADR 0064](docs/adr/0064-configuration-file-format.md) — configuration is TOML in `mwl.toml`, not
  INI.** INI has no specification and one value type; four directives the ADRs already specify were
  booleans, lists or repeated records encoded as strings, including a `:`-joined `script.spawn` that
  cannot express a Windows absolute path. `ini_set`/`ini_get`/`ini_restore` are now
  `Core\Config::set`/`::get`/`::restore` (ADR 0011 forced that regardless), and `Core\Config` has a row in
  [`docs/spec/01-core-library.md`](docs/spec/01-core-library.md) § 16. ADR 0005's registry, changeability
  classes, ceilings and overlay are all unchanged — only the syntax and the names moved.
- **[ADR 0065](docs/adr/0065-third-party-attribution-and-mwl-info.md)** — generated third-party
  attribution and `mwl info`. This closes the previous session's backlog item about an untracked
  `tools/gen-attribution.py`: it was that ADR's generator, arriving before its number settled.

## Next

**Stage 3 — the rest of `Core` §§ 1–12.** `examples/core.mwl` now stops at its *first* line. Three things
stand between here and its six frozen output lines, and each is a session-sized slice:

1. **Integer `/` and `%`.** `mwl-codegen` still refuses `Div`/`Mod` because `sdiv` traps the whole process
   on a zero divisor, which is a request-isolation failure. The throw path exists, so this is a checked
   divisor plus a `Terminator::Throw` — and ADR 0007 § 4's own arithmetic table for the `int`/`uint` rows.
   `examples/core.mwl` line 3 needs `%`. Smallest of the three; a good opener.
2. **An options-shape argument, and an optional parameter.** `Core\Arr::range(int, int, {step?: int})` and
   `Core\Arr::sort(array<T>, {by?: callable, …})` both need `mwl_stdlib::registry::CoreTy` to express a
   shape and the arity check to accept a missing trailing argument. ADR 0063 R2 makes this the shape of
   *every* optioned member, so it is worth doing properly once — see `mwl-stdlib`'s own known gap 3.
   `mwl_types::ty::Ty::Shape` and ADR 0036 § 3's width subtyping already exist on the checker side.
3. **`Core\Str` rows.** `upper`, `lower`, `join`, `split`, `replace`, `padStart`, `padEnd`, `upperFirst`,
   `length` — what `examples/core.mwl` and `examples/report.mwl` call between them. **ADR 0009 must be
   settled by measurement first** (the loop goal's standing decision says so, and the spec agrees):
   implement both granularities behind one seam, write the figure into
   `a_grapheme_index_costs_more_than_a_code_point_index` in `benches/abi-probe/tests/perf_guards.rs`, pick
   the default it justifies, move the ADR to Accepted — *before* writing `Core\Str`'s conformance cases.
   Only `length`/`at`/`slice` actually depend on it; the other rows do not, so they can land first.

Then `Core\Arr::map`/`sort` (both are `callable`-taking rows the closure work already unblocked — note
`map`'s `array<U>` return substitutes an unbound `U` to `mixed` today, which `Core\Str::join` will refuse,
so `map` needs a decision about that before it is useful), then `crates/mwl-test` and `mwl test`.

## Backlog

Each crate's own module doc is the home for its known gaps; these are the ones worth surfacing.

- **`$fn(...)` direct invocation has no lowering**, and ADR 0031 § 3's self-name is parsed and ignored, so
  a recursive closure reports an undefined name. Native `Core` code is the only caller today. Direct
  invocation needs a decision first: `callable` is opaque (ADR 0031 § 4), so `$f(3)` has no argument types
  to check against and no result type but `mixed`.
- **A closure capturing or declaring a `&$x` binding panics naming itself** — the cell it addresses is the
  caller's, and a closure may outlive the call that staged it. Same shape as the generator's own refusal.
- **A helper that fails leaks its borrowed temporaries** — `Lowering::release_call_temporaries` sits on the
  normal edge only. One more caller for the owned-temporaries stack `landing_block`'s gap already needs.
- **`Iterator::current()` does not throw** before the first `advance()` or after one returned `false`;
  ADR 0053 § 1 says it must. `lower_generator_current`'s doc owns the gap.
- **A generator with a `&$x` parameter, or a `Ty::Ref` binding live across a `yield`, panics naming
  itself**, and **`static::`/`new static()` inside a generator body** reaches `Lowering::lsb`'s panic.
- **A `foreach` key binding over an array is unchecked** — `array<T>` records no key type;
  `mwl_types::expr::check_foreach_key`'s doc names `array<K, V>` as the prerequisite.
- **A name in type position is lowered twice**, so `E0441`/`E0442`/`E0303` print twice. Cosmetic, loud.
- **A by-reference call is lowered in two positions only**, and **a callee that throws never reaches its
  by-reference copy-back** — `Lowering::pending_refs` and `mwl_ir::Ty::Ref` own both.
- **An array-element write through a hooked property** (`$obj->hooked[0] = v`) panics naming itself.
- **ADR 0014's `PropertyObserver` half** is untouched, and **a hook on a `static`/`readonly` property is
  not refused** — `mwl_types::check::check_property_hooks` is where that diagnostic would go.
- **An integer *into* an enum** is the last unimplemented conversion row (ADR 0010 § 5), and **a checked
  conversion throws a `RuntimeError`, not an `ArithmeticError`** (ADR 0007 § 4 names the closer one).
- **Virtual dispatch through a base-typed local** — `mwl-codegen`'s known gap 1, narrowed not closed — and
  **`new static()` calls no constructor** when the statically resolved chain declares none.
- **`finally` misses two exits**, and **a `catch` clause inside a `namespace` block will not resolve**
  (`catch_clause_type` takes the written text; fix it as `instanceof` did).
- **`for`/`switch`/`match` are still unlowered.** `Terminator::Switch` was built general for `switch`.
- **ADR 0043's `by`-delegation** is unimplemented, and `mwl_types::conformance` exempts any class using one
  *whole* because of that — closing the gap and narrowing that exemption go together.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of exactly the kind CLAUDE.md
  forbids. The one doc in the repo genuinely owed a trim.
- **Nothing on disk parses `mwl.toml` yet** — ADR 0064 is a decision, and M6 is where it is built. Nothing
  in M4/M4S depends on it.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`.

Eight tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, including inside a `python - <<'PY'` script: a
  trailing `\` silently vanishes (joining a wrapped Rust string into one long line) and `\\` becomes one
  backslash. This bit again this session, twice, on `\`-continued Rust strings. The reliable shape is:
  **Write the new code to a file under `target/` with the Write tool**, then have a short heredoc Python
  script splice that file into place. The splice script itself only ever matches on plain text.
- **Escaped `\"` inside a Rust test fixture does not survive a heredoc either.** Editing an existing test
  string is a job for the Edit tool, not a Python replace.
- **`gen` is a reserved keyword in Rust 2024.** `generator` is what `mwl_ir::lower` uses.
- **`rustfmt` rewrites a `"...\n..."` literal in a test into a real multi-line string**, so a patch
  matching `\\n` inside a fixture stops matching after the first `cargo fmt`.
- **Write a commit message to `target/`, never the repo root** — and **check `git show --stat` after every
  commit**: `git add -A` swept an unrelated untracked file into one this session (see the backlog).
- `python`, not `python3`, is what is on `PATH` here.
- **`wsl.exe` needs PowerShell**, not the Bash tool, and a **script file** — an inline `bash -lc "…"`
  mangles. The full leg is `wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh`. For a narrower check,
  write a one-off script plus fixture under `target/` and run that — `target/wsl-closure-check.sh` is the
  pattern, and it is what caught this session's leak. Worth doing for **any** hand-written retain/release.
- `cargo insta test --accept -p <crate>` is installed (note `test --accept`). Read the diffs first; a
  renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two
  minutes — run it in the background.
