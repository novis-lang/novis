# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open. (That file's *gaps that sit on the path* list is
a snapshot from before the loop started and several entries are now closed — trust the plan and the crate
gap lists over it.)

**Conformance is 190 against 250, differential 82 against 60.** Stage 4 fails on that one count alone, and
growing the conformance corpus is the whole remaining gap. The `.mwlt` format — every section,
`--EXPECTF--`'s escapes, why a case runs in a subprocess as `case.mwl`, and why a `--ORACLE--` case skips
when PHP is absent — is `crates/mwl-test`'s own module doc, routed from
[`docs/adr/README.md`](../adr/README.md) § *Where to look*.

**Never put `--ORACLE--` in a `tests/conformance/` case.** The WSL leg has no PHP, so an oracle section
makes the runner *skip the whole case* there — it would subtract from the very count Stage 4 measures.
Verify against PHP while authoring (a scratch twin under `.agent-tmp/`), then either drop the section or
put the case in `tests/differential/` where the oracle is the point. Cross-suite duplication of a program
is the existing pattern, not a smell.

**Writing cases is still the fastest bug-finder in the repo — it found one more, still open.** An
**abandoned generator never runs the `finally` it is suspended inside**: `break` out of a `foreach` parks
the state machine at its `yield` and nothing resumes it, where PHP resumes it in a return-like mode on
destruction. `mwl-ir`'s known gap 18 has the shape of the fix and why it is a design call (it is the first
thing in that crate that looks like a destructor, which ADR 0028 says MWL has none of). The two halves that
*do* agree with PHP are pinned in `tests/differential/iter/`.

**Every fixture is green on both legs**, byte for byte, and `valgrind --leak-check=full` clean.
Stages 1–3 are done. **Nothing is blocked.**

**PHP 8.5 is on `PATH` under Windows but not inside the WSL distro**, so the Linux leg skips the
`--ORACLE--` cases and runs the `--ORACLE-DIVERGES--` ones. That is deliberate: the native leg's
`min_passing = 60` in [`loop-goal.toml`](loop-goal.toml) is what catches an oracle that goes missing where
it matters. Installing PHP in WSL is the user's call, not a session's.

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

**Keep growing `tests/conformance/`.** It is the only thing between the loop and Stage 4, it needs no new
machinery, and it splits cleanly across sessions. M4's **Verify** list in
[the plan](../implementation-plan.md) is the specification; the *Verification* section of ADRs 0014, 0023,
0028, 0046 and 0069 each names its own required cases. Ground already covered densely: array key order and
append indices, `try`/`catch`/`finally` ordering, generators, interface defaults, `Core\Str`/`Core\Arr`.
Thinner: property hooks, `clone`, `instanceof`, `lateinit`, `Comparable`, and compile-error cases (an
`--EXPECTF-ERROR--` case is cheap and pins a diagnostic's wording).

**What a case cannot use yet** — each sits in the named crate's known-gap list, and every one panics or
refuses rather than failing cleanly, so writing around them saves an edit cycle: `&&`/`||`/ternary anywhere
but a declaration initializer, a `return` value, an assignment right-hand side or a condition (`mwl-ir`
gap 5, so not inside an `echo` argument); a compound assignment `$x += 1` in any form (gap 16); a *nested*
array write `$grid[0][1] = v` (gap 6 — reading it, at any depth, is fine, and `$obj->rows[] = v` does
work); a static property **read or write** (gap 6); a class constant's *value* (it types as `mixed`, as
does `Core\Reflect::class`); **any nullable `?T`** — parameter, return or local (gap 3's `null` half, which
panics in `lower_checked_ty`); a **bitwise** operator (`&`/`|`/`^`/`<<`/`>>`/`~`); `$f(...)` on a
closure-typed local (gap 9 — it types as `mixed`, so write the `fn` literal at the call site); an object
literal `{a: 1}` and a shape-typed parameter; a **named or spread call argument**, so
`new LogicError("x", previous: $e)` does not lower; `do`/`while`; an implicit `Stringable` in `echo` or `.`
(gap 12 — call `toString()` explicitly); `bool as int`, `mixed as string` and `as ?T` (gaps 4 and 3);
integer `/`; `for`/`switch` (gap 1). In `mwl-codegen`: `<`/`>` over two strings, `==`/`===` over two
*objects* or two *enum values* (compare `$a as int`), integer `+`/`-`/`*` **wrapping instead of throwing on
overflow** (gap 8 — do not freeze a case around it), and an `int` mixed with a `float` in one operator
(write `0.0 - 1.5`). `<=>` lowers over two *objects* through `Comparable` but not over two `int`s. In
`mwl-types`: a parameter typed `Stringable`/`Comparable` has no method to call and `instanceof Stringable`
panics (the empty reserved-interface roster); an **enum case is refused as a parameter default**
(`mwl-types`' `defaults` module doc says why); `lateinit` is compile-time-checked and refused on a
`string`/`int` property. **A `catch` binding is function-scoped**, so two clauses on one `try` cannot both
bind `$e`. Traps that produce a confusing panic or a `mixed` rather than an error: `as` binds tighter than
every binary operator, `instanceof` and unary minus, so write `($a > $b) as string`, `(($a + $b) as
string)` and `(-7) as string`; a **bare array literal in a `foreach` head types as `mixed`** — bind it to a
declared `array<T>` local first; a `foreach` key binding must be declared `string` even over a list
(ADR 0007 § 5); `Core\Arr::map` through a *variable* of type `callable` yields `array<mixed>`.
`Core\Str::length` and `Core\Arr::count` return `uint`. `false as string` is the empty string.
`Iterable<T>`'s member is `iterate(): Iterator<T>`; `Iterator<T>`'s two are `advance(): bool` and
`current(): T`, not PHP's five; an enum declares bare `Case,` lines, not `case`.

Two smaller slices, either of which fits a session on its own:

- **`autoload`** ([ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)) — the smaller of
  the two remaining M1 grammar additions; the duration literal (0070) follows.
- **More `Core` §§ 1–2 rows.** Everything left in § 1 that needs no new registry machinery is a row plus a
  body; `Core\Str::slice` is the exception and waits on `mwl-ir` gap 3.

**When a session is short**, one migration-table pass instead: pick a domain from
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md)'s *Not yet classified* list, run
`python tools/check-migration.py --report`, classify it against the spec. It is 31% classified.

## Backlog

- **An abandoned generator skips the `finally` it is suspended inside** — `mwl-ir`'s known gap 18, the one
  PHP divergence the corpus has found and not closed.
- **A `finally` still does not run when a `catch` clause's own body throws** — `mwl-ir`'s known gap 2.
- **A compound assignment (`+=`, `.=`, …) does not lower** — `mwl-ir`'s known gap 16; every PHP program
  writes one, so it is the widest single hole left in the surface a conformance case can reach.
- **Nullable `?T` has no IR representation at all** — gap 3; probably the next-widest hole after that.
- **Class-member `private`/`protected` is not enforced at all** — `mwl-types`' own gap list.
- **The reserved `Comparable`/`Stringable` interfaces carry no member signatures** — same gap list.
- **ADR 0047's M2 checker row** is unblocked and is what removes `E_LITERAL_TYPE_UNCHECKED`.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. Use the Write tool, or `python tools/splice.py <target> <old> <new>` with both blocks written to
  `.agent-tmp/`. A throwaway `.agent-tmp/*.py` script run with `python` is fine for a bulk edit.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. Keep a PHP twin beside it — a
  disagreement with PHP is the bug you are hunting.
- **Another agent may be editing this repo at the same time.** **Check the ADR directory for the next free
  number immediately before writing one**, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed).
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh` (run it in the background; it takes minutes). One
  fixture: `tools/leak-check.sh <paths>`.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it. It short-circuits on
  the first failure, so it stops at Stage 4's conformance count today.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links, the second kind being the one that
  works on Windows and 404s on Linux.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
