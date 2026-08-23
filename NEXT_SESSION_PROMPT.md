# Next session prompt

## State

**M3 is complete.** **M4 and M4S run together as one loop** — read
[`.claude/loop-goal.md`](.claude/loop-goal.md) first: it is authoritative for the acceptance list, for the
ten standing decisions already settled with the user, and for the gaps that sit on the path. Do not
re-open any of those decisions. Re-run the Linux leg by hand with
`wsl.exe -- bash /mnt/<drive>/<repo>/.claude/wsl-acceptance.sh` from a **PowerShell** call.

**The last session was planning, not code.** It set the loop up and recorded the decisions it needed:
the exception surface moved to the spec's § 10 tree (ADR 0020 § 1 now points there), the spec's
*Milestones* line was corrected to §§ 1–12, M4's `Core` bullet moved to M4S, M4's CLI-program bullet was
scoped to its computational half, and the `mwl-core`/`mwl-stdlib` name collision was settled on
`mwl-stdlib`. Eight new frozen fixtures exist under `examples/`, and `.claude/loop.ps1` now checks six
ordered stages ending in a valgrind sweep.

## Next: the object representation, which everything else waits on

`mwl_ir::Ty::Object` refcounts nothing — no field layout, no instance dispatch, no array
(`mwl-codegen` known gap 1). Until that lands, nothing else either milestone owes has a site to attach to.

A reasonable order, each step unblocking the next:

1. **Object layout and allocation** in `mwl-runtime`, then `FieldGet`/`FieldSet` and an instance `Call`
   in `mwl-codegen`. `mwl_ir::lower::lower_file` skips interfaces and enums for the same missing reason.
2. **The class hierarchy at runtime**, which `instanceof` and a typed `catch` both need —
   `mwl_ir::lower::is_global_throwable` accepts exactly three names today, and `lower_try` still panics on
   a second `catch` clause and on `finally`.
3. **The ordered-hash array with COW.** Build the refcount-1 in-place check reachable from the start: it
   is the whole cost argument for [ADR 0063](docs/adr/0063-core-api-conventions.md) R3, and
   `a_refcount_one_array_member_mutates_in_place` in `benches/abi-probe` has to measure it.
4. **`crates/mwl-test` and the `.mwlt` runner**, early rather than late — Stage 4's two suites hold most
   of this loop's coverage, and hand-writing them as PowerShell assertions instead is the trap.

`examples/objects.mwl`, `arrays.mwl` and `errors.mwl` are Stage 1's whole target; get those three green
before looking at anything in Stage 2 or 3.

## Backlog

Ordered roughly by how cheap each is next to the bulk above.

**Rides on the object representation:**

- Integer `Div`/`Mod`, still refused in `mwl-codegen` because `sdiv` traps on a zero divisor. The throw
  path exists now, so this is a checked divisor plus a `Terminator::Throw`. `examples/core.mwl` needs it.
- A `throw` satisfying the return check: a method declared `: int` whose body always throws counts as
  returning.
- ADR 0043 `by`-delegation resolution + `E_DELEGATE_TYPE_MISMATCH`, then `E_INTERFACE_MEMBER_CONFLICT`.

**Independent of it:**

- **A `THROWN` still leaks a temporary in flight** inside the expression that threw
  (`mwl_ir::lower::Lowering::landing_block`); it needs an owned-temporaries stack threaded through
  `lower_expr`, and Stage 6's valgrind leg will find it. A `FATAL` leaking the frame's locals is
  deliberate while a `FATAL` ends the request — revisit when M5/M6 give a request an arena.
- `for` and `switch` in `mwl-ir` (`for` reuses `LoopFrame`; `switch` needs the N-way terminator ADR 0053's
  generator resumption also wants, so the two pair naturally).
- ADR 0018's `BRANCH` probe — the last of that ADR's three sites still not emitted.
- A string literal still allocates per evaluation (`mwl-runtime` known gap 3 / `mwl-codegen` known gap 4).
- The safepoint/debug-flags loads use `MemFlagsData::with_notrap()`; they must become atomic when M5
  introduces a watchdog thread. Named in `mwl-codegen`'s `emit::ctx_word`.
- [ADR 0061](docs/adr/0061-compile-time-autoload-and-program-discovery.md) parser half, then the `mwl-hir`
  resolver half. Three new `E03xx` codes, starting at **`E0315`**.
- **Docs-only:** `crates/mwl-ir/src/lib.rs`'s module doc has become a slice-by-slice changelog of exactly
  the kind CLAUDE.md forbids. It is the one doc in the repo genuinely owed a trim pass.

## Standing rules for this repo

Read `CLAUDE.md` first and follow its *Where to look* table rather than reading `docs/` breadth-first.
Every fact has exactly one home; if two documents state the same thing, the one CLAUDE.md names is
authoritative and the other is a bug — including this file, which is overwritten, never appended to.
After editing any doc, run `python .claude/brief.py --check`: it enforces a per-entity cap and names the
exact line and byte count to cut. That is a one-line fix, never a reason to run a trim pass.

Two tooling notes worth keeping:

- **The Bash tool eats a backslash inside a heredoc**, and `sed -i` patterns holding a backslash, backtick
  or em-dash silently no-op. Use Write and Edit for any content with escapes.
- **`wsl.exe` paths need PowerShell, not the Bash tool**, which rewrites `/mnt/d/…` first.
