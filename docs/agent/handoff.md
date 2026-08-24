# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, every fixture
`valgrind --leak-check=full` clean. Five registry mechanisms work end to end: an options bag, a union
parameter, a callback-bound result type, a `Core`-owned enum and an absent option. `examples/core.mwl` runs
its line 11 sort and stops at `Core\Str::length`. **Nothing is blocked.**

**Seven `Core` additions were decided with the user, and one ADR per session is writing them up.**
[`core-additions.md`](core-additions.md) is the queue; an entry is deleted as its ADR lands, and its numbers
are stable, so a gap means that one is done. **Entry 1 landed as
[ADR 0071](../adr/0071-derived-codecs.md)** — derived codecs. Six remain, in order: `Core\Task`, the
`[schedule]` block, HTTP defaults in both directions, `Core\RateLimit`, observability export, compile-time
routing.

**What 0071 settled**, since two of the remaining entries lean on it:

- `#[Json\Derive]`/`#[Db\Derive]` generate a `Codec` from a class's **declared property list**, and every
  non-skipped field must also be a same-named, same-typed **constructor parameter** — so a decode is an
  ordinary `new` and no invariant is bypassed. `#[Json\Field(name?, skip?)]` is the only per-field control.
- Required / optional / nullable are three existing concepts: a parameter default makes a key optional, `?T`
  makes `null` legal, and they are independent. No naming policy, no omit-null, no validation.
- A decode throws **once**, carrying every failed field. `ParseError` and `DbError` gained
  `issues: array<Core\Issue>` where `type Core\Issue = {path: string, message: string};` — no new exception
  class, spec § 10's closed set unchanged in shape.
- **The general rule the queue needed:** a **compiler-recognized** attribute is matched **nominally**,
  against a closed `Core`-owned list; ADR 0046 § 4's *structural* `Core\Attributes` retrieval is untouched.
  That is what stops a route scan double-registering a framework's own `#[Route]` literals, so **entry 7's
  "real interaction to resolve" is already resolved** — it only has to add `Core\Route` to the list.
- `Core\Serialize` gets no derive (it already handles every object, R17), and `#[Db\Derive]` is
  one-directional (no `toRow`, no generated `INSERT`).

**The standing rule has landed too**: *domain logic is an existing first-class Rust crate; compiler passes
and scheduler primitives are ours* — [`docs/adr/README.md`](../adr/README.md) § *Decisions taken at project
start*, plus its `AGENTS.md` bullet. Nothing else is owed from that session.

**Declined in the same session**, reasons recorded in the queue file so they are not re-argued: typed
templates, `mwl migrate`, Markdown in `Core` (it becomes a first-party extension), and a DI container at any
tier — whose real fix is user-defined generics, an open ADR 0007 question.

**The `Core` roster review** before that changed docs only, nothing on disk: `Core\Time` lost its
relative-date string, [ADR 0070](../adr/0070-duration-literals.md) is new and is **M1 grammar that is not
built**, six duplicates went and eight gaps were filled.
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md) is **31% classified**, held by
`python tools/check-migration.py` in CI.

**Two questions the user has not answered**, both recorded in place rather than guessed:

1. **`Uri::parseQuery`/`buildQuery` and PHP's bracket arrays** (`a[]=1`, `a[b]=c`). Marked *Open* in
   [spec § 12](../spec/01-core-library.md); the same answer has to serve `Core\Request::query`, so it is
   an M8 decision, not an M4S one.
2. **`levenshtein`** is classified `dropped` (byte-oriented, wrong on UTF-8) with no member. A UTF-8
   `Str::editDistance` is a live candidate the user has not ruled on.

**The dependency-update policy** is [ADR 0068](../adr/0068-dependency-currency-and-the-version-contract.md)
plus [`dependency-update.md`](dependency-update.md): prototyping regime, a bump is free — but **the sweep is
a pass the user fires by hand**. Never start it, and never bump as a side effect.

## Next

**One ADR per session from [`core-additions.md`](core-additions.md), in its order** — next is **entry 2,
`Core\Task`** (`::all`/`::map` with `{limit, deadline}`, plus `::afterResponse`; explicitly not a durable
queue). Check the ADR directory for the next free number immediately before writing: another agent may have
taken 0072.

**The code path is unchanged and runs in parallel with that.** Four re-opened M1 grammar slices — `decimal`
(0054), literal/enum-case type atoms (0047), `autoload` (0061), the duration literal (0070) — one per
session, smallest first, and build `decimal` before the duration literal because both land on the same
numeric-literal-suffix branch of the lexer from opposite sides: `19.99m` must stay an *unknown token*
([0054](../adr/0054-decimal-scalar-type.md) § 2 rejected the `m` suffix outright), while `30m` becomes one
`DurationLiteral` token. Then **settle [ADR 0009](../adr/0009-string-and-bytes.md) § 2 by measurement** and
land `Core\Str::length`/`at`/`slice`, the last thing between `examples/core.mwl` and its frozen six lines:
implement both granularities behind one seam, write the figure into
`a_grapheme_index_costs_more_than_a_code_point_index` in `benches/abi-probe/tests/perf_guards.rs`, and pick
the default it justifies. After that: more cheap § 1–2 rows, then `crates/mwl-test` and `mwl test`.

**When a session is short**, one migration-table pass instead: pick a domain from that file's *Not yet
classified* list, run `python tools/check-migration.py --report`, classify it against the spec.

## Backlog

- **A closure literal written as a call argument leaks its environment object when that call throws** — the
  landing-block gap in `mwl-ir`'s known gap 2; it needs an owned-temporaries stack threaded through
  `lower_expr`.
- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed. `mwl_types::core_lib`'s module doc owns the trust rule.
- **A `Core` member cannot return a union or `?T`, and a `?T` parameter cannot be declared** — one
  `mwl_ir::ty::Ty::Mixed` representation question under both, `mwl-ir`'s known gap 3.
- **Only a written `fn` literal binds a callback's result type** — `mwl_types::generics`' own *Known gap*.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5. `a_typed_arithmetic_loop_contains_no_call` counts every `call` in
  `Bench::sum`, so read that guard's claim before adding a raise block.
- **A `Core` helper cannot name the class it throws** — every `Fault::thrown` becomes `RuntimeError`, so
  spec § 10's `LogicError` is unreachable from `mwl-stdlib`. `mwl_runtime::mwl_raise_new` is the shape.
- **`<`/`>` over two `string`s is unlowered** — `mwl-codegen` refuses a `Gt` over representation `Str`.
- **`$a + $b` and `$a += $b` over two arrays still have no diagnostic** — ADR 0069 § 2 requires one naming
  `Arr::underlay`; M4's arithmetic checking is where it belongs, and M4's **Verify** list now says so.
- **Still-large files, deliberately not split yet** — `mwl-syntax`'s `parser.rs` (split it as the first
  step of ADR 0040's M4B resilient-parse work) and `mwl-types`'s `expr.rs`. Revisit `mwl-stdlib`'s `arr.rs`
  past ~2,500 lines.

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
- **Another agent may be editing this repo at the same time.** **Check the ADR directory for the next free
  number immediately before writing one**, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh`. One fixture: `tools/leak-check.sh <paths>`, whose
  header says why to run it for **any** new refcount edge.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links, the second kind being the one that
  works on Windows and 404s on Linux.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file,
  so `src/foo.rs` becoming `src/foo/mod.rs` means `src/snapshots/` moves to `src/foo/snapshots/`.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
