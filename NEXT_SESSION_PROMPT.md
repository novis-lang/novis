# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`.claude/loop-goal.md`](.claude/loop-goal.md) first — it is
authoritative for the acceptance list and for the ten standing decisions already settled with the user;
do not re-open any of them. The plan's status block says what is on disk and what is open.

**ADR 0014 § 1's property hooks went end to end this session.** `examples/hooks.mwl` now prints
`6`/`20`/`Counter(10)` and fails only on its last line. The shape, owned by the doc comments rather than
summarised here:

- A hooked property's access is a **call**, not a field touch. Each hook body compiles to an ordinary
  function under `mwl_types::signatures::hook_label`'s label (`Counter::$doubled::get`), takes the same
  implicit receiver in parameter slot 0 every method takes, and is reached through the same
  `InstKind::Call` — so it costs no new instruction, no calling convention and no dispatch-table entry.
  `mwl_ir::lower::lower_property_hook` is the lowering.
- `ExprInfo::HookedProperty` is recorded *instead of* `ExprInfo::Property`, carrying both accessor labels
  because one entry answers a read and a write.
- `mwl_types::Ctx::current_hook` is the one exception: inside `$p`'s own hooks, `$this->p` is the backing
  slot. That is what makes a hook that transforms a stored value terminate.
- **Every hooked property is still backed** — MWL has no virtual/backed split.
  `mwl_types::signatures::PropertyHooks`' doc comment owns that decision and what it spends; ADR 0014's
  *Revisiting* no longer lists it as open.
- `is_aliasing_read` became `Lowering::aliasing_read`, because a `get`-hooked read looks syntactically
  like a slot read but produces a fresh, already-owned value like any other call's.

## Next

**By-reference parameters (`int &$slot`) — the last line of `examples/hooks.mwl`, and the last of
Stage 1.** `Adder::bump($n)` still prints `n=1`; it must print `n=6`. This is M4's `references (&$x)`
bullet, and it needs a representation decision made and recorded before any code — per
`.claude/loop-goal.md`'s standing "decide and record; never `BLOCKED` for a design call", in
`mwl_ir::ty`'s or `mwl_runtime`'s own module doc, **not** a numbered ADR.

ADR 0007 § 1 already settles the *meaning* (both sides declare the same type), so the open question is
purely what a `&T` parameter is at the ABI level. Last session weighed two models without implementing
either; start from this rather than re-deriving it:

- **True aliasing (a pointer to the caller's own storage).** Most faithful to PHP, and the most
  expensive: every local that is ever the target of `&` has to be demoted out of SSA into an addressable
  stack slot, which touches phis, `Env` and the refcount policy. `&$arr[0]` is worse than hard — an
  array element has no stable address under ADR 0007 § 5's copy-on-write.
- **A caller-staged one-slot temporary** — the caller allocates a stack slot, stores the current value,
  passes its address, and copies back after the call; the callee treats the parameter as a pointer, so
  every read is a load and every write a store. No SSA demotion anywhere: the pointer is loop-invariant,
  so nothing in the callee needs a phi it did not already need, and the caller's holder can be a local,
  a property or an array element without any of them becoming addressable. The one divergence from PHP
  is that a callee that **throws** never reaches the copy-back — recoverable by doing the load-and-write-back
  on `emit_fallible`'s error edge as well as its normal one, which is where to look first rather than
  accepting the divergence.

Whichever is chosen, the work is: `Ty::Ref(T)` (or equivalent) in `mwl_ir::ty`; the new load/store/slot
instructions and their `mwl-codegen` arms (Cranelift `StackSlot`/`stack_addr`); `mwl_types` checking that
a `&` argument is a writable place and that its type matches exactly; and the refcount policy for a
refcounted `&T`, which needs stating explicitly in the same doc comment as the representation.

After Stage 1, **Stage 2** is `examples/iterate.mwl`: ADR 0053's generators and the two iteration
interfaces, blocked first on `implements Iterable<int>` not parsing (a generic interface in an
`implements` clause).

## Backlog

Each crate's own module doc is the home for its known gaps; these are the ones worth surfacing.

- **An array-element write through a hooked property** (`$obj->hooked[0] = v`) panics naming itself —
  `mwl_ir::lower::Lowering::write_back_array`. The copy-on-write separation would have to be written back
  through the `set` hook, and no PHP-compatible rule for that exists yet.
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
