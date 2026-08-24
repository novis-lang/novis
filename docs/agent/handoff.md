# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open.

**Stages 1 and 2 plus `examples/report.mwl` are green on both legs**, byte for byte, every fixture
`valgrind --leak-check=full` clean. Five registry mechanisms work end to end: an options bag, a union
parameter, a callback-bound result type, a `Core`-owned enum and an absent option. `examples/core.mwl` runs
its line 11 sort and stops at `Core\Str::length`. **Nothing is blocked.**

**Seven `Core` additions were just decided with the user, and none of them is written up.** The session
asked which everyday userland needs of a modern web application belong in `Core`, filtered every candidate
through [ADR 0051](../adr/0051-standard-library-tiers.md) § 2's six tests and
[ADR 0060](../adr/0060-application-security-protocols.md) § 2's three, and settled each one by hand.
[**`core-additions.md`**](core-additions.md) is the queue: one entry per decision, in the order they should
be written, each naming the ADRs it amends, the Rust crate that does the work, and the design points the
session deliberately did *not* settle. **An entry is deleted as its ADR lands**, so the file is a queue and
never a second home for a rule. In one line each:

1. **Derived codecs** — `#[Json\Derive]` generates `Core\Json\Codec`/`Core\Db\Codec` from declared property
   types; a bad payload reports **every** failed field, not the first.
2. **`Core\Task::all`/`::map`** with `{limit, deadline}`, plus **`::afterResponse`** for work that runs once
   the response is on the wire. Explicitly not a durable queue.
3. **A `[schedule]` cron block** in `mwl.toml`, firing a `spawn script`; **`scope` is mandatory**, no default.
4. **HTTP defaults safe and finite in both directions** — `[http.cors]`/`[http.headers]`/`[http.cookies]`
   with secure defaults on and every directive `Runtime`-class, and a `Core\Http\Client` with no spelling
   for "wait forever" plus opt-in jittered retry.
5. **`Core\RateLimit`** over the shared store, filling the hole
   [ADR 0059 § 4](../adr/0059-cross-request-state-is-explicit.md) already named. Edge/flood limiting is
   dropped to the proxy.
6. **Observability export** — automatic request/DB/GC/spawn metrics and W3C trace-context propagation, plus
   a three-member `Core\Metrics`; exporter is feature-gated Native, cardinality capped.
7. **Compile-time routing** — `#[Route]` discovered while compiling, `match()` and reverse `url()` only, no
   dispatch. Carries one real interaction to resolve against ADR 0046's *structural* attribute retrieval.

**Also decided: a standing rule.** *Domain logic must be an existing first-class Rust crate; compiler passes
and scheduler primitives are ours.* If no crate exists for something with an external specification, the
feature is not built. Its home is a paragraph in *Decisions taken at project start* in
[`docs/adr/README.md`](../adr/README.md), beside "Pure-Rust dependencies by default", plus an `AGENTS.md`
bullet. Neither is written yet. **Declined in the same session**, with reasons recorded in the queue file so
they are not re-argued: typed templates, `mwl migrate`, Markdown in `Core` (it becomes a first-party
extension), and a DI container at any tier — whose real fix is user-defined generics, an open ADR 0007
question.

**The whole `Core` roster had a member-by-member review with the user** in the session before that, against
[ADR 0063](../adr/0063-core-api-conventions.md)'s twenty rules. Nothing on disk was invalidated. What
changed, all of it in docs: `Core\Time` lost its relative-date string (no `DateTime::shift`, no `strtotime`
grammar; CLDR patterns, not PHP's `date()` letters); [ADR 0070](../adr/0070-duration-literals.md) is new —
`30s`, `1h30m`, one token, always `Duration`, and it is **M1 grammar that is not built**; six duplicates
removed and eight gaps filled; `Arr::flatten` split into `flatten`/`flattenDeep`.
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md) is new, one row per PHP built-in, **31%
classified**, held by `python tools/check-migration.py` in CI.

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

**One ADR per session from [`core-additions.md`](core-additions.md), in its order**, starting with **0071,
derived codecs** — it is the largest, it folds into two accepted ADRs (0063 § 4 and 0067 § 6) and two spec
sections, and it is the one whose shape constrains the others. Check the ADR directory for the next free
number immediately before writing: 0070 landed mid-session from another agent, which is why this one is
0071. Write the standing rule's two paragraphs (README + `AGENTS.md`) with whichever ADR you write first,
since it is a sentence rather than a session.

**The code path is unchanged and runs in parallel with that.** Four re-opened M1 grammar slices — `decimal`
(0054), literal/enum-case type atoms (0047), `autoload` (0061), the duration literal (0070) — one per
session, smallest first, and build `decimal` before the duration literal because their `m` handling meets at
the same lexer position. Then **settle [ADR 0009](../adr/0009-string-and-bytes.md) § 2 by measurement** and
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
