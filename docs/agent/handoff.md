# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, every fixture
`valgrind --leak-check=full` clean. Five registry mechanisms work end to end: an options bag, a union
parameter, a callback-bound result type, a `Core`-owned enum and an absent option. `examples/core.mwl` runs
its line 11 sort and stops at `Core\Str::length`. **Nothing is blocked.**

**M1 was re-opened for four grammar additions; two are built.** `decimal`
([ADR 0054](../adr/0054-decimal-scalar-type.md)) landed grammar *and* its M2 checker rows; its runtime half
is `mwl-ir`'s known gap 15. **Literal and enum-case type atoms
([ADR 0047](../adr/0047-literal-and-enum-case-types.md)) landed grammar only** — the checker refuses all
three atoms by name (`E_LITERAL_TYPE_UNCHECKED`) rather than widening them to their base type, which that
ADR's *Verification* section owns. Two grammar slices remain, one per session, in the plan's M1 section:
`autoload` (0061) and the duration literal (0070).

**The docs track is finished and owes nothing.** All seven `Core` additions decided with the user are
written, as **ADRs 0071–0077**: derived codecs, `Core\Task`, `[[schedule]]`, HTTP defaults both directions,
`Core\RateLimit`, observability export, compile-time routing. Nothing about them needs summarising here;
each ADR's *Verification* section names what its milestone owes, and
`python tools/brief.py --where <keyword>` routes to whichever one a topic belongs to. Four cross-cutting
rules those seven produced, because they will be reached for from outside their own ADRs:

- A **compiler-recognized** attribute is matched **nominally**, against a closed `Core`-owned list — now
  five names, `#[Json\Derive]`/`#[Json\Field]`/`#[Db\Derive]`/`#[Db\Field]` plus `#[Core\Route]`
  ([0071](../adr/0071-derived-codecs.md) § 1). ADR 0046 § 4's *structural* `Core\Attributes` retrieval is
  untouched.
- Every `mwl.toml` block now has one routing table: [0064 § 2a](../adr/0064-configuration-file-format.md),
  naming the ADR that argues each block's directives.
- **Typed `callable` signatures** ([ADR 0007](../adr/0007-explicit-type-system.md) § 3) now block four
  separate features — enough that the deferral itself should be re-argued rather than re-deferred. The
  four are named in [0077](../adr/0077-compile-time-routing.md)'s *Revisiting*.
- [ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) § 3's program scan has a second
  caller (the route table), under the identical opt-in rule and cache consequence.

**Declined with the additions**, reasons recorded in each owning ADR's *Alternatives rejected*: typed
templates, `mwl migrate`, Markdown in `Core` (a first-party extension instead), a DI container at any tier
(its real fix is user-defined generics), edge/flood rate limiting ([0075](../adr/0075-core-ratelimit.md)
§ 4), and a durable job queue ([0072](../adr/0072-core-task-structured-concurrency.md) § 6).

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

**The code path is the only track left.** Either of two next slices fits a session:

- **`autoload`** ([ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)) — the smaller of
  the two remaining M1 grammar additions; the duration literal (0070) follows, and meets `decimal` at the
  lexer (that item's own plan paragraph says how).
- **[ADR 0009](../adr/0009-string-and-bytes.md) § 2, settled by measurement**, then
  `Core\Str::length`/`at`/`slice` — the last thing between `examples/core.mwl` and its frozen six lines.
  Implement both granularities behind one seam, write the figure into
  `a_grapheme_index_costs_more_than_a_code_point_index` in `benches/abi-probe/tests/perf_guards.rs`, and
  pick the default it justifies.

After those: more cheap § 1–2 rows, then `crates/mwl-test` and `mwl test`.

**When a session is short**, one migration-table pass instead: pick a domain from
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md)'s *Not yet classified* list, run
`python tools/check-migration.py --report`, classify it against the spec. It is 31% classified and is the
one doc item still open.

## Backlog

- **ADR 0047's M2 checker row** is now unblocked and is what removes `E_LITERAL_TYPE_UNCHECKED`; that
  ADR's *Verification* names the step § 4's own table understates (a literal *expression* must type as its
  literal type first, or nothing satisfies one of these types without an `as`).
- **A closure literal written as a call argument leaks its environment object when that call throws** — the
  landing-block gap in `mwl-ir`'s known gap 2; it needs an owned-temporaries stack threaded through
  `lower_expr`.
- **An unregistered *member* on a registered `Core` class types as `mixed`, silently** — so
  `Core\Str::lenght($s)` is not diagnosed. `mwl_types::core_lib`'s module doc owns the trust rule.
- **A `Core` member cannot return a union or `?T`, and a `?T` parameter cannot be declared** — one
  `mwl_ir::ty::Ty::Mixed` representation question under both, `mwl-ir`'s known gap 3.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5. `a_typed_arithmetic_loop_contains_no_call` counts every `call` in
  `Bench::sum`, so read that guard's claim before adding a raise block.
- **`$a + $b` and `$a += $b` over two arrays still have no diagnostic** — ADR 0069 § 2 requires one naming
  `Arr::underlay`; M4's arithmetic checking is where it belongs, and M4's **Verify** list now says so.
- **Still-large files, deliberately not split yet** — `mwl-syntax`'s `parser.rs` (split it as the first
  step of ADR 0040's M4B resilient-parse work) and `mwl-types`'s `expr.rs`.

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
