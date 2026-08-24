# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open.

**`mwl test` exists and both suites are green: 52 conformance cases, 34 differential**, against thresholds
of 250 and 60, so Stage 4 still **fails on corpus size alone**. The `.mwlt` format — every section,
`--EXPECTF--`'s escapes, why a case runs in a subprocess as `case.mwl`, and why a `--ORACLE--` case skips
when PHP is absent — is `crates/mwl-test`'s own module doc, routed from
[`docs/adr/README.md`](../adr/README.md) § *Where to look*.

**Every fixture is green on both legs**, byte for byte, every one `valgrind --leak-check=full` clean.
Stages 1–3 are done. **Nothing is blocked.**

**PHP 8.5 is on `PATH` under Windows but not inside the WSL distro**, so the Linux leg skips the
`--ORACLE--` cases and runs the `--ORACLE-DIVERGES--` ones. That is deliberate: the native leg's
`min_passing = 60` in [`loop-goal.toml`](loop-goal.toml) is what catches an oracle that goes missing where
it matters. Installing PHP in WSL is the user's call, not a session's.

**[ADR 0009](../adr/0009-string-and-bytes.md) is Accepted in full**, § 2 included. The seam is
`mwl_stdlib::granularity`, whose module doc owns the dependency choice and the one gap — nothing caches a
count yet, which the plan's M4S paragraph carries. The figures live only in
`a_grapheme_index_costs_more_than_a_code_point_index` (`benches/abi-probe/tests/perf_guards.rs`).

**M1 was re-opened for four grammar additions; two are built.** `decimal`
([ADR 0054](../adr/0054-decimal-scalar-type.md)) landed grammar *and* its M2 checker rows; its runtime half
is `mwl-ir`'s known gap 15. **Literal and enum-case type atoms
([ADR 0047](../adr/0047-literal-and-enum-case-types.md)) landed grammar only** — the checker refuses all
three atoms by name (`E_LITERAL_TYPE_UNCHECKED`). Two grammar slices remain, one per session, in the plan's
M1 section: `autoload` (0061) and the duration literal (0070).

**The docs track is finished and owes nothing** — all seven `Core` additions are ADRs 0071–0077. Four
cross-cutting rules from that track, because they get reached for from outside their own ADRs:

- A **compiler-recognized** attribute is matched **nominally**, against a closed `Core`-owned list of five
  names ([0071](../adr/0071-derived-codecs.md) § 1). ADR 0046 § 4's *structural* retrieval is untouched.
- Every `mwl.toml` block has one routing table: [0064 § 2a](../adr/0064-configuration-file-format.md).
- **Typed `callable` signatures** ([ADR 0007](../adr/0007-explicit-type-system.md) § 3) now block four
  features, named in [0077](../adr/0077-compile-time-routing.md)'s *Revisiting* — re-argue the deferral
  rather than re-defer it.
- [ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md) § 3's program scan has a second
  caller (the route table), under the identical opt-in rule.

**Two questions the user has not answered**, both recorded in place: `Uri::parseQuery`'s handling of PHP
bracket arrays (marked *Open* in [spec § 12](../spec/01-core-library.md)), and whether a UTF-8
`Str::editDistance` should exist where `levenshtein` was dropped.

**The dependency sweep is a pass the user fires by hand** —
[ADR 0068](../adr/0068-dependency-currency-and-the-version-contract.md) plus
[`dependency-update.md`](dependency-update.md). Never start it, never bump as a side effect.

## Next

**Keep growing `tests/conformance/` and `tests/differential/`.** It is the only thing between the loop and
Stage 4, it needs no new machinery, and it splits cleanly across sessions. M4's **Verify** list in
[the plan](../implementation-plan.md) is the specification; the *Verification* section of ADRs 0014, 0023,
0028, 0046 and 0069 each names its own required cases. Write a `--ORACLE--` twin whenever the behaviour is
one MWL claims is PHP-compatible, and an `--ORACLE-DIVERGES--` with its one-line reason whenever it is not.

**What a case cannot use yet** — each already sits in the named crate's known-gap list, and every one of
these panics or refuses rather than failing cleanly, so writing around them saves a whole edit cycle:
`&&`/`||`/ternary anywhere but a declaration initializer, a `return` value, an assignment right-hand side
or a condition (`mwl-ir` gap 5, so not inside an `echo` argument); `$f(...)` on a closure-typed local (gap
9); a class constant's *value*, and a write to a static property (gaps 6 and the `mwl-types` list); `as`
on a `Stringable` (gap 12); `bool as int` and `as ?T` (gaps 4 and 3); `<`/`>` over two strings
(`mwl-codegen`); `for`/`switch` (gap 1). Two precedence traps that produce a confusing panic rather than a
parse error: `as` binds tighter than comparison, `instanceof` and unary minus, so write `($a > $b) as
string` and `(-7) as string`. And a `foreach` key binding must be declared `string` even over a list —
ADR 0007 § 5 — which is what a list case should assert rather than avoid.

Two smaller slices, either of which fits a session on its own:

- **`autoload`** ([ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)) — the smaller of
  the two remaining M1 grammar additions; the duration literal (0070) follows.
- **More `Core` §§ 1–2 rows.** Everything left in § 1 that needs no new registry machinery is a row plus a
  body; `Core\Str::slice` is the exception and waits on gap 3 below.

**When a session is short**, one migration-table pass instead: pick a domain from
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md)'s *Not yet classified* list, run
`python tools/check-migration.py --report`, classify it against the spec. It is 31% classified.

## Backlog

- **A `foreach` key binding declared `int` panics in `mwl-ir` instead of getting a diagnostic** —
  `mwl-types`' own known-gap list; ADR 0007 § 5 makes it always wrong.
- **ADR 0047's M2 checker row** is unblocked and is what removes `E_LITERAL_TYPE_UNCHECKED`.
- **A `Core` member cannot return a union or `?T`, and a `?T` parameter cannot be declared** — `mwl-ir`'s
  known gap 3, and what `Core\Str::slice` waits on.
- **A closure literal written as a call argument leaks its environment object when that call throws** —
  `mwl-ir`'s known gap 2; it needs an owned-temporaries stack threaded through `lower_expr`.
- **Integer `+`/`-`/`*` wrap rather than throw, and integer `/` is refused two phases deep** —
  `mwl-codegen`'s known gaps 8 and 5; read `a_typed_arithmetic_loop_contains_no_call` before adding a raise.
- **`$a + $b` and `$a += $b` over two arrays still have no diagnostic** — ADR 0069 § 2 requires one.
- **Still-large files, deliberately not split yet** — `mwl-syntax`'s `parser.rs` (split it as the first
  step of ADR 0040's M4B work) and `mwl-types`'s `expr.rs`.

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
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed).
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh`. One fixture: `tools/leak-check.sh <paths>`.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it. It short-circuits on
  the first failure, so it stops at Stage 4's case counts today.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree. A scratch program under `.agent-tmp/` run with
  `mwl run` is the fastest way to find out whether a shape lowers before writing a case around it.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links, the second kind being the one that
  works on Windows and 404s on Linux.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
