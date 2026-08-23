# Hotspot-splitting pass

A standing prompt, fired by hand like [docs/agent/doc-cleanup.md](doc-cleanup.md) — never from inside
the loop. Three mechanical file splits, in order. Every one is a **pure move**: no behaviour change, no
signature change, no new `pub`. `cargo test` must pass with the same test count before and after each.

## Before you start

1. **The loop must not be running.** `tools/loop.py` sessions edit `registry.rs` and `lower.rs` on almost
   every iteration, so this pass and the loop cannot share a tree. Check `.loop/log.md` for a
   `## run ended` line as the last entry; if there is none, stop and tell the user rather than proceeding.
2. Run `python tools/brief.py`, read `AGENTS.md`, and follow its *Where to look* table.
3. **Locate everything by symbol name, not by line number.** Any line number in this file was true when it
   was written and the loop has moved on since.
4. Multi-line Rust edits go through `python tools/splice.py <target> <old> <new>`, per `AGENTS.md`'s
   *Commands* section. For whole-block moves, `git mv` plus the Write tool is usually cleaner than a splice.

Commit each split separately, verifying `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D
warnings` and `cargo fmt --check` between them. Do not batch the three into one commit.

## Split 1 — `mwl-stdlib`'s two per-member hotspots

**Why:** every one of the ~190 `Core` members still owed by `docs/spec/01-core-library.md` §§ 1–12 edits
the same two places, so two sessions adding two different domains always conflict. This is what makes a
parallel domain lane possible at all.

`crates/mwl-stdlib/src/registry.rs` holds one flat `pub const CLASSES` with every class's members inlined,
and `crates/mwl-stdlib/src/lib.rs`'s `symbols()` holds one match arm per member.

- Give each domain module its own `pub const CLASS: CoreClass`, holding that class's `CoreMethod` rows.
- Reduce `CLASSES` to a list of those consts, in the spec's own § order:
  `pub const CLASSES: &[CoreClass] = &[crate::str::CLASS, crate::arr::CLASS];`
- Give each domain module a `pub(crate) fn address(symbol: &str) -> Option<*const u8>` answering only for
  its own symbols, and make `symbols()` chain them with `.or_else(...)`, keeping the existing panic for an
  unregistered symbol.

Adding a new domain must then be **one line in `registry.rs` and one in `lib.rs`**, plus its own new file.
`every_registered_member_has_an_implementation_address` and `no_two_members_share_a_symbol` must both still
pass untouched — they read `CLASSES`, whose contents do not change.

`crates/mwl-types/src/core_lib.rs` is **not** part of this: its `seed` is a generic loop over `CLASSES` and
does not grow per member. Leave it alone.

## Split 2 — the test modules two sessions fight over

**Why:** collision surface, not file size. `crates/mwl-types/src/check.rs` is ~380 lines of code carrying a
~2,700-line inline `mod tests`, and `crates/mwl-codegen/tests/compile_and_run.rs` is ~1,900 lines that every
new feature appends to.

- Move `check.rs`'s tests into `crates/mwl-types/tests/`, split by rule area — casing, constructor
  initialization, taint/secret, conversions, and whatever else the existing test names already group into.
  Anything they reach that is currently `pub(crate)` needs a decision: prefer keeping the test inline over
  widening visibility, and say so in the commit message where you do.
- Split `compile_and_run.rs` by feature area the same way. `a_second_script_runs_after_a_contained_helper_panic`
  is named in `docs/agent/loop-goal.md` Stage 5 — it must keep its name and still run.

Test count before and after must match exactly. Report both numbers in the commit message.

## Split 3 — `crates/mwl-ir/src/lower.rs`

**Why:** the file every remaining machinery slice lands in — `for`, `switch`, `match`, `$fn(...)`,
ADR 0043's `by`-delegation, and the owned-temporaries stack the `THROWN` temporary leak needs. It is ~6,900
code lines, and `impl<'a> Lowering<'a>` alone is ~5,500 of them in one block. Every loop session pays that
in fresh context.

Rust allows one `impl` to be split across modules in the same crate, so this is a move and nothing else.
Turn it into a `lower/` directory, roughly:

| module | holds |
|---|---|
| `mod.rs` | the `Lowering` struct, `new`/`finish`, the `emit_*` primitives, the refcount helpers |
| `stmt.rs` | `lower_stmt` and its dispatch |
| `expr.rs` | `lower_expr` and its dispatch |
| `control.rs` | `lower_if`, `lower_while`, `lower_foreach`, `lower_break`, `lower_continue` |
| `exception.rs` | `lower_throw`, `lower_try`, `lower_catch_clauses`, the `finally` path |
| `generator.rs` | `lower_yield`, `gen_target`, `finish_generator`, `GenFrame` |
| `call.rs` | the call lowering paths and `ArgSig` |

Adjust the grouping if the code disagrees with this sketch — the sketch is not authoritative, the code is.
Keep every function private exactly as it is now.

In the same commit, fix `crates/mwl-ir/src/lib.rs`'s module doc, which `AGENTS.md`'s *Writing docs here*
rules forbid and `docs/agent/loop-goal.md` already names as the one doc in the repo genuinely owed a trim: it
is a slice-by-slice changelog, and that history belongs in `git log`.

## Out of scope

- **`crates/mwl-syntax/src/parser.rs`.** It wants the same treatment, but [ADR 0040](../adr/0040-vscode-deep-tooling-and-resilient-parsing.md)
  adds a second, error-recovering entry point to that crate in milestone M4B. Split it as the first step of
  *that* work, so the reorganization happens once.
- **`crates/mwl-types/src/expr.rs`.** Same reasoning: `Core`-owned enums and `ConstArg::Null` are both due
  to edit it next.
- **`emit.rs`, `arr.rs`, `str.rs`.** Still navigable. Revisit `arr.rs` when it passes ~2,500 lines.
- **Any behaviour change, any doc-cleanup pass, any `docs/agent/handoff.md` rewrite.** If you find a bug
  while moving code, note it in `## Backlog` there and leave it.

## When you are done

Tell the user what each split changed, the before/after test
counts, and whether the parallel-lane design in `docs/agent/coordinator.md` § *Why not a coordinator
conversation at all?* is now worth revisiting — it currently says concurrent sessions race, and Split 1 is
the thing that makes a bounded number of them stop racing.
