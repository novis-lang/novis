# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, every fixture
`valgrind --leak-check=full` clean. Five registry mechanisms work end to end: an options bag, a union
parameter, a callback-bound result type, a `Core`-owned enum and an absent option. `examples/core.mwl` runs
its line 11 sort and stops at `Core\Str::length`. **Nothing is blocked.**

**The dependency-update policy just landed** — [ADR 0068](../adr/0068-dependency-currency-and-the-version-contract.md)
plus the procedure in [`dependency-update.md`](dependency-update.md). Two things it says that change how you
work: **we are still in the prototyping regime**, so updating any dependency is free and needs no
classification until 0.1.0 ships; and **the sweep is a pass the user fires by hand** — never start it, and
never bump a dependency as a side effect of unrelated work. If you notice something stale, say so and carry
on. Nothing on disk changed, and no code work depends on it.

**M8's database design is settled and recorded** — [ADR 0067](../adr/0067-core-db.md) plus
[spec § 18](../spec/01-core-library.md). It folded into 0024, 0041, 0051 and 0058, so those bodies moved;
none of it touches M4/M4S work or anything on disk today. Do not re-open it.

**An earlier housekeeping pass split the three files sessions used to collide in** — `mwl-stdlib`'s registry
is one line per class with each domain owning its own rows, `mwl-types`'s checker tests and `mwl-codegen`'s
end-to-end tests are one file per area, and `mwl-ir`'s `lower.rs` is a `lower/` directory. 1077 tests.

## Next

**Three re-opened M1 grammar slices, then ADR 0009 § 2.** Three ADRs accepted *after* M1 was reported done
add grammar M1 owns, none of it built, each blocking a checker slice M2 is already scheduled for. The plan's
M1 section lists all three; take them one per session, smallest first:

1. **`decimal`** ([ADR 0054](../adr/0054-decimal-scalar-type.md)) — a keyword in `mwl_syntax::token`, a
   `TypeAtom`, and a fractional literal that is untyped until placed rather than immediately `float`. The
   word does not appear anywhere in `crates/` today. ADR 0054's *Verification* has the M1 fixture list.
2. **Literal and enum-case type atoms** ([ADR 0047](../adr/0047-literal-and-enum-case-types.md)) — a
   `StringLiteral`/`IntLiteral` atom, unions of them, `?"a"` sugar.
3. **`autoload`** ([ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)) — the two
   file-scope forms whose grammar [`docs/spec/00-overview.md`](../spec/00-overview.md) § 2 already fixes.

Then **settle [ADR 0009](../adr/0009-string-and-bytes.md) § 2 by measurement** and land
`Core\Str::length`/`at`/`slice` — the last thing between `examples/core.mwl` and its frozen six lines. The
ADR is Accepted now; only § 2's granularity is open. Implement both granularities behind one seam, write
the figure into `a_grapheme_index_costs_more_than_a_code_point_index` in
`benches/abi-probe/tests/perf_guards.rs`, and pick the default it justifies. After that: more cheap § 1–2
rows (`Str::lines`/`chunk`/`replaceAll`, `Arr::reverse`/`fill`/`sortByKey`), then `crates/mwl-test` and
`mwl test` (loop goal, Stage 4).

## Backlog

- **A closure literal written as a call argument leaks its environment object when that call throws** — the
  landing-block gap in `mwl-ir`'s known gap 2; it needs an owned-temporaries stack threaded through
  `lower_expr`.
- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed. `mwl_types::core_lib`'s module doc owns the trust rule; a `Core`
  enum case is already out of this hole and shows the shape a fix would take.
- **A `Core` member cannot return a union or `?T`, and a `?T` parameter cannot be declared** — one
  `mwl_ir::ty::Ty::Mixed` representation question under both, `mwl-ir`'s known gap 3.
- **Only a written `fn` literal binds a callback's result type** — `mwl_types::generics`' own *Known gap*.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5. `a_typed_arithmetic_loop_contains_no_call` counts every `call` in
  `Bench::sum`, so read that guard's claim before adding a raise block.
- **A `Core` helper cannot name the class it throws** — every `Fault::thrown` becomes `RuntimeError`, so
  spec § 10's `LogicError` is unreachable from `mwl-stdlib`. `mwl_runtime::mwl_raise_new` is the shape.
- **`<`/`>` over two `string`s is unlowered** — `mwl-codegen` refuses a `Gt` over representation `Str`.
- **Still-large files, deliberately not split yet** — `mwl-syntax`'s `parser.rs` (split it as the first
  step of ADR 0040's M4B resilient-parse work, so it happens once) and `mwl-types`'s `expr.rs`. Revisit
  `mwl-stdlib`'s `arr.rs` past ~2,500 lines.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. For a multi-line Rust edit, write the old and new blocks to files under `.agent-tmp/` with the
  Write tool, then `python tools/splice.py <target> <old> <new>`.
- **Another agent may be editing this repo at the same time** — one overwrote this very file mid-session.
  `brief.py` says so loudly when `.loop/running` exists. **Stage your own paths explicitly, check
  `git show --stat` after committing, and re-read a shared doc immediately before rewriting it.**
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh`. One fixture: `tools/leak-check.sh <paths>`, whose
  header says why to run it for **any** new refcount edge.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **After moving or renaming any doc, `python tools/check-links.py`** — broken *and* mis-cased relative
  links, the second kind being the one that works on Windows and 404s on Linux. Advisory, exits 0 always.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file,
  so `src/foo.rs` becoming `src/foo/mod.rs` means `src/snapshots/` moves to `src/foo/snapshots/`.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
