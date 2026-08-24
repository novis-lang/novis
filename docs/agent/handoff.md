# Next session prompt

## State

**The M4 + M4S loop goal is met.** `python tools/loop.py --goal-only` reports GOAL REACHED: every check in
[`loop-goal.toml`](loop-goal.toml) passes — all sixteen fixture runs byte for byte on Windows *and* under
WSL against a Linux build, both suites over their thresholds (conformance 260 against 250, differential 82
against 60), every named guard, and the `valgrind --leak-check=full` sweep clean over all thirteen acyclic
fixtures. [`loop-goal.md`](loop-goal.md) is authoritative for what that list means and for the standing
decisions already settled with the user; do not re-open any of them.

**The last session was docs-only: [ADR 0078](../adr/0078-config-reload-and-control-socket.md) landed**,
decided with the user in conversation. `mwl.toml` stops being read only at boot — `mwl ctl reload` replaces
the whole config snapshot over a **local socket only** (no TCP listener, no token, no TLS; the socket's
owner and mode are the authentication), validating the replacement whole before publishing it. Every
directive gains a `Reload`/`Boot` field orthogonal to its changeability class, and reload *names* the `Boot`
keys it could not apply. The extension set joins one `env_hash` carried by both compiled-unit cache keys,
which makes extension reload fall out of ADR 0017's existing lazy revalidation with no invalidation pass —
and closes a latent hole where an on-disk artifact compiled against one extension set could be reused
against another. Folds applied to 0003, 0005, 0017, 0042, 0048, 0064 and the plan's cache sketch.
**Nothing was implemented**: it builds in M6 (snapshot + `env_hash`), M7 (socket + `mwl ctl`) and M9
(extension reload), and each milestone's *Verify* list now says what it owes.

**Goal-reached is not spec-complete, and the plan's status block says which is which.** `Core` §§ 1–12 is
still `Core\Arr` × 20 and `Core\Str` × 19 against a much longer spec, and the plan's *Open now* field lists
the rest — three `Core` shapes that need registry machinery, two re-opened M1 grammar slices, ADR 0047's
checker row, unenforced `private`/`protected`, and the 31%-classified migration table. **Nothing is
blocked.** A new loop needs a new `loop-goal.toml`, not another session against this one.

## Next

**More `Core` §§ 1–2 registry rows**, the largest remaining body of work with no machinery in the way.
[The spec](../spec/01-core-library.md) § 2 is the work list; reachable *today* is `chunk` (the first member
returning a nested `array<array<T>>`, plus a `preserveKeys` bag `reverse` already proves) and
`flatten`/`flattenDeep` — whose spec signature `flatten(array<T> $a): array<T>` binds `T` to the *outer*
element type, so decide and record what the return type is before writing the row. Three shapes are **not**
reachable and each says so in `mwl-stdlib`'s own gap list: a `?T` return (`first`, `last`, `keyOf`,
`firstKey`, `lastKey`), a variadic `...$layers` (ADR 0069's `overlay`/`underlay`/`appendAll`, the widest gap
in § 2), and a union *return*. `contains`/`diff`/`intersect`/`unique` additionally want one strict-identity
comparison over two `Value`s, which `mwl-runtime` does not have yet and which is worth its own slice —
decide object identity there and record it in that crate's module doc.

**Every new `Core` member owes a `.mwlt` case in the same session.** `mwl-stdlib`'s
`every_part_one_member_has_a_conformance_case` fails naming the member otherwise; that test's own module doc
says what it enumerates.

**Second choice: keep growing `tests/conformance/`** — no machinery, splits cleanly across sessions. M4's
*Verify* list in [the plan](../implementation-plan.md) is the specification, and ADRs 0014, 0023, 0028, 0046
and 0069 each name their own required cases. Covered densely: array key order, `try`/`catch`/`finally`,
generators, interface defaults, `Core\Str`/`Core\Arr`, property hooks, `clone`, `instanceof`, `Comparable`,
`lateinit`, and the whole refused-construct family. Thinner: `bytes`, `decimal`, attributes, `uint` edges.
**Read `mwl-ir`'s, `mwl-codegen`'s and `mwl-types`' known-gap lists before writing a batch** — every entry
there is a shape a case cannot use yet, and most panic or refuse rather than failing cleanly. The traps that
cost the most time are not gaps at all: `as` binds tighter than every binary operator, `instanceof` and
unary minus, so write `($a > $b) as string`; a bare array literal in a `foreach` head types as `mixed`; a
`foreach` key binding must be declared `string` even over a list; `Core\Str::length` and `Core\Arr::count`
return `uint`; a `catch` binding is function-scoped, so two clauses on one `try` cannot both bind `$e`.

**Never put `--ORACLE--` in a `tests/conformance/` case** — the WSL leg has no PHP, so an oracle section
makes the runner *skip the whole case* there, subtracting from the very count Stage 4 measures. Verify
against PHP while authoring (a scratch twin under `.agent-tmp/`), then drop the section or put the case in
`tests/differential/`. The `.mwlt` format is `crates/mwl-test`'s module doc.

**Two questions the user has not answered**, both recorded in place: `Uri::parseQuery`'s handling of PHP
bracket arrays (marked *Open* in [spec § 12](../spec/01-core-library.md)), and whether a UTF-8
`Str::editDistance` should exist where `levenshtein` was dropped.

## Backlog

- **`Core\Arr`'s three ADR 0069 combination members need a variadic parameter** — no `CoreTy` states one,
  and they are the widest hole in § 2 (`mwl-stdlib`'s gap list).
- **A compound assignment (`+=`, `.=`, …) does not lower** — `mwl-ir`'s gap 16; every PHP program writes
  one, so it is the widest single hole left in the surface a conformance case can reach.
- **Nullable `?T` has no IR representation at all** — `mwl-ir`'s gap 3; the next-widest hole after that.
- **An abandoned generator skips the `finally` it is suspended inside** — `mwl-ir`'s gap 18, the one PHP
  divergence the corpus has found and not closed.
- **`new` on an `abstract` class is not refused, and `$n->foo()` on a scalar receiver panics `mwl-ir`** —
  both missing `mwl-types` diagnostics.
- **Class-member `private`/`protected` is not enforced at all** — `mwl-types`' own gap list, whose reserved
  `Comparable`/`Stringable` interfaces also still carry no member signatures.
- **ADR 0078 is decided but unbuilt** — it is scheduled into M6/M7/M9 and needs nothing before then. A
  network-reachable control surface is deferred, not rejected; that ADR's *Revisiting* records the design so
  it is not re-derived.

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
  lowers**, and is worth doing before writing a batch of cases around it. Keep a PHP twin beside it.
- **Another agent may be editing this repo at the same time.** **Check the ADR directory for the next free
  number immediately before writing one**, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed).
  The dependency *sweep* is a pass the user fires by hand (ADR 0068); never start it as a side effect.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh` (background it; minutes). One fixture:
  `tools/leak-check.sh <paths>`. PHP 8.5 is on `PATH` under Windows but **not** inside WSL, deliberately.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.
