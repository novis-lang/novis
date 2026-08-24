# Next session prompt

## State

**M4 + M4S run as one loop.** Read [`docs/agent/loop-goal.md`](loop-goal.md) first — authoritative for the
acceptance list and the standing decisions already settled with the user; do not re-open any of them. The
plan's status block says what is on disk and what is open. (That file's *gaps that sit on the path* list is
a snapshot from before the loop started and several entries are now closed — trust the plan and the crate
gap lists over it.)

**Conformance is 161 against 250, differential 76 against 60.** Stage 4 fails on that one count alone, and
growing the conformance corpus is the whole remaining gap. The `.mwlt` format — every section,
`--EXPECTF--`'s escapes, why a case runs in a subprocess as `case.mwl`, and why a `--ORACLE--` case skips
when PHP is absent — is `crates/mwl-test`'s own module doc, routed from
[`docs/adr/README.md`](../adr/README.md) § *Where to look*.

**Writing cases is still the fastest bug-finder in the repo — it found three more**, all fixed, all
PHP-verified, and all documented where they live:

- **A `break`/`continue` out of a protected region silently skipped its `finally`.** `LoopFrame::try_depth`
  records the `try_stack` height the loop body started at; `Lowering::run_finallys_above` unwinds exactly
  the regions the jump leaves, innermost out. `lower_try`'s known-gap list now names only the one real
  remaining hole (a `catch` body that itself throws).
- **A `foreach` *anywhere* inside a generator produced code Cranelift's verifier rejected** — the loop used
  the pre-loop array/cursor value rather than the header phi, and a generator's resume edge re-enters that
  header with the value reloaded from its frame. Both `lower_foreach` and `lower_foreach_cursor` now read
  the reserved names back out of `header_env`; the comment there says why.
- **PHP's simple-syntax array interpolation `"$row[key]"` panicked in `cook_string_literal`** — the parser
  makes a bareword offset an `ExprKind::Str` over an *unquoted* span. That branch is now handled; a digit
  run stays a string on purpose, since a numeric key normalises to an int one anyway.

`CodegenError::Cranelift` now renders its source's `Debug` as well as its `Display` — a verifier rejection's
`Display` is the bare words "Verifier errors" and says nothing about which lowering is wrong.

**Expect more of the same: probe a shape with a scratch `.mwl` before writing cases around it.**

**Every fixture is green on both legs**, byte for byte, every one `valgrind --leak-check=full` clean, and
both new refcount edges were leak-checked against purpose-built fixtures. Stages 1–3 are done.
**Nothing is blocked.**

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
0028, 0046 and 0069 each names its own required cases. A differential twin is still worth writing when the
behaviour is one MWL claims is PHP-compatible — that suite is over its bar, not finished. **Generators just
became far more testable**: a `foreach` inside one now works, so pipelines, generators over arrays and
generators over other generators are all fair game.

**What a case cannot use yet** — each sits in the named crate's known-gap list, and every one panics or
refuses rather than failing cleanly, so writing around them saves an edit cycle: `&&`/`||`/ternary anywhere
but a declaration initializer, a `return` value, an assignment right-hand side or a condition (`mwl-ir`
gap 5, so not inside an `echo` argument); a compound assignment `$x += 1` in any form (gap 16); a *nested*
array write `$grid[0][1] = v` (gap 6 — reading it, at any depth, is fine); a static property **read or
write** (gap 6); a class constant's *value* (it types as `mixed`); **any nullable `?T`** — parameter,
return or local (gap 3's `null` half, which panics in `lower_checked_ty`); a **bitwise** operator
(`&`/`|`/`^`/`<<`/`>>`/`~`, `mwl-ir`'s binary-operator arm); `$f(...)` on a closure-typed local (gap 9); an
object literal `{a: 1}` and a shape-typed parameter (gap 5's expression list); a **named or spread call
argument**, so `new LogicError("x", previous: $e)` does not lower and `Throwable`'s constructor takes one
argument; `do`/`while` (`mwl-ir`'s statement list); an implicit `Stringable` in `echo` or `.` (gap 12 —
call `toString()` explicitly); `bool as int` and `as ?T` (gaps 4 and 3); integer `/` (no single `int|float`
IR representation); `for`/`switch` (gap 1). In `mwl-codegen`: `<`/`>` over two strings, `===` over two
*objects* (`Eq` over `Ty::Object`), integer `+`/`-`/`*` **wrapping instead of throwing on overflow**
(gap 8 — do not freeze a case around it), and gap 9's two refusals — an `int` mixed with a `float` in one
operator (write `0.0 - 1.5`) and `===` over two enum values (compare `$a as int`). `<=>` lowers over two
*objects* through `Comparable` but not over two `int`s. Also: a parameter typed `Stringable`/`Comparable`
has no method to call, and `instanceof Stringable` panics in `mwl-ir` — both are the empty
reserved-interface roster, in `mwl-types`' gap list. `lateinit` is checked at compile time, so a
read-before-write is a diagnostic rather than the ADR 0038 throw, and it is refused on a `string`/`int`
property (class- or interface-typed only). **A `catch` binding is function-scoped**, so two clauses on one
`try` cannot both bind `$e` — use distinct names. Three spelling traps that produce a confusing panic
rather than a parse error: `as` binds tighter than every binary operator, `instanceof` and unary minus, so
write `($a > $b) as string`, `(($a + $b) as string)` and `(-7) as string`; a `foreach` key binding must be
declared `string` even over a list (ADR 0007 § 5); and `Core\Arr::map` through a *variable* of type
`callable` yields `array<mixed>` — write the `fn` literal at the call site when the element type matters.
`Core\Str::length` and `Core\Arr::count` return `uint`. `false as string` is the empty string, so render a
`bool` through a `fn`/method returning `"y"`/`"n"` when a case needs to see it. `Iterator<T>`'s two members
are `advance(): bool` and `current(): T`, not PHP's five; an enum declares bare `Case,` lines, not `case`.

Two smaller slices, either of which fits a session on its own:

- **`autoload`** ([ADR 0061](../adr/0061-compile-time-autoload-and-program-discovery.md)) — the smaller of
  the two remaining M1 grammar additions; the duration literal (0070) follows.
- **More `Core` §§ 1–2 rows.** Everything left in § 1 that needs no new registry machinery is a row plus a
  body; `Core\Str::slice` is the exception and waits on `mwl-ir` gap 3.

**When a session is short**, one migration-table pass instead: pick a domain from
[`docs/spec/02-php-migration.md`](../spec/02-php-migration.md)'s *Not yet classified* list, run
`python tools/check-migration.py --report`, classify it against the spec. It is 31% classified.

## Backlog

- **A `finally` still does not run when a `catch` clause's own body throws** — `mwl-ir`'s known gap 2; the
  clause body's frame names no handler on purpose, so the throw skips straight out.
- **A compound assignment (`+=`, `.=`, …) does not lower** — `mwl-ir`'s known gap 16; every PHP program
  writes one, so it is the widest single hole left in the surface a conformance case can reach.
- **Nullable `?T` has no IR representation at all** — gap 3; it blocks a whole family of ordinary PHP
  shapes and is probably the next-widest hole after compound assignment.
- **Class-member `private`/`protected` is not enforced at all** — `mwl-types`' own gap list; a PHP-visible
  divergence, and it wants one pass keyed on the accessing class.
- **The reserved `Comparable`/`Stringable` interfaces carry no member signatures** — same gap list; filling
  the roster is what makes a `Stringable` parameter and `instanceof Stringable` work.
- **ADR 0047's M2 checker row** is unblocked and is what removes `E_LITERAL_TYPE_UNCHECKED`.

## Standing rules for this repo

Read `AGENTS.md` first — the rules file, harness-neutral, at the root. Route a topic with
`python tools/brief.py --where <keyword>` rather than reading `docs/` breadth-first. Every fact has one
home; a second copy is a bug — including this file, overwritten never appended to. **An ADR's body states
the current rule**; if it disagrees with a cross-link, the body is the bug. **Nothing measures doc length**,
so never spend an iteration trimming one. Follow `AGENTS.md` § *Session workflow*: work, verify once, docs
+ handoff, commit, **stop** — no second `cargo` pass after the commit. Tooling notes:

- **The Bash tool eats a backslash inside a heredoc**, and an apostrophe-heavy one can fail to parse at
  all. This bites hardest in a `python - <<'PY'` one-liner rewriting a fixture: `"\\n"` arrives as a real
  newline and the replacement silently matches nothing. Use the Write tool, or
  `python tools/splice.py <target> <old> <new>` with both blocks written to `.agent-tmp/`.
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
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh`. One fixture: `tools/leak-check.sh <paths>`.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it. It short-circuits on
  the first failure, so it stops at Stage 4's conformance count today — run `wsl-acceptance.sh` by hand
  after touching codegen, since that is the leg the short-circuit hides.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links, the second kind being the one that
  works on Windows and 404s on Linux.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>` (note
  `test --accept`); a renamed test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe`
  takes over two minutes — run it in the background.
