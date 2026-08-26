# Handoff

## State

**Conformance is at 538 of 600, and it is the only frontier left.** Verify is green (1597 cargo tests,
74 suites, 538 conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself,
so after a green `verify.py` there is nothing else to run (playbook, *Running things*).

**The differential gate is met at 159, but `gaps.py --differential` is NOT empty — it is 9**, and the
claim that it was empty is withdrawn from the plan and from here. This session fixed `gaps.py`'s
`CLASS_RE`, which matched only a class naming itself inline and so read **7 of the tree's 27 classes**;
the 9 are `Core\Time::now`/`monotonic`/`sleep`/`fromIso`/`parse`/`at` and
`Core\Encoding::encodeText`/`decodeText`/`isValidText`. They are candidates, not work: `now`, `sleep`
and `monotonic` have no oracle a case could freeze, which is exactly the judgement gaps.py says is the
session's job. The gate itself is unaffected — it counts cases, and 159 still clears 150.

**This session added no library code. It cut the loop's fixed cost, measured off
`.loop/logs/20260826-205949-*`.** That run's sessions were 38 tool calls and 8.4 minutes each against
98 and ~20 before the previous pass, so the earlier work held; what was left was rediscovery. Five
changes, each removing a call the tail or head was spending on something the tree already knew:
`session.py --wrap` now commits the docs it writes (**9 of 19 sessions** closed with a hand-rolled
`git add docs/agent/handoff.md docs/agent/playbook.md docs/implementation-plan.md && git commit` after
the wrap had already written all three); `--template` hands back every stale count as an applicable
`## plan-edit:` and prints the playbook's headings (13 `grep -n "^## " playbook.md` and 12
`grep -n "<count>" implementation-plan.md` calls between them); `gaps.py --coverage` ranks classes by
cases per member, which is the `ls tests/` plus `grep -n 'name: "'` pair sessions ran ~10 times;
`orient.py`'s closing block drops the `--dry-run` step and states the debug CLI is already built, and
`loop.py` builds it before session 1 so that is true from the first one. Expected saving ~7 of 38 calls.

**`loop-stats.py` was measuring batching wrongly.** It reported "NOTHING WAS EVER BATCHED" off
calls-per-message, while 42% of shell calls chained 3.3 commands each — 1.91 commands per shell call
over that run. It now reports both, so a goal author is not sent after a saving already taken.

## Next group

Unchanged and untaken — the conformance slices below are the same three the previous session left, and
`Core\Math` is closed but for the first of them. All three add a new file under `tests/conformance/core/`
and all three are the *agreement* shape from conventions.md. `docs/spec/01-core-library.md` §§ 3 and 7
own the rules. The tolerance spelling, the well-conditioned direction and the exact-on-both-legs rows are
playbook bullets under *Writing a test case* — do not re-derive any of them.

- [ ] **`format` and `round` agree wherever both name the same precision** — `Core\Math::format($n,
      {decimals: $d})` renders what `Core\Math::round($n, {precision: $d})` answers, on every row of
      a table, and parts from it only in the options `round` has no opinion about (the separators,
      whose defaults are MWL's and not `number_format`'s). `crates/mwl-stdlib/src/math.rs:110`
      (`round`), `:313` (`format`), `:449` (`FORMAT_OPTIONS`, where the empty group separator is
      decided).
- [ ] **`split` and `join` are inverses through `normalize`'s normal form** — `join` of what `split`
      returned is `normalize($p)` on every row of a table, `split` never yields an empty segment
      however many separators were repeated, and the round trip is asserted separator-free or
      through `Core\Str::replace($p, Core\Path::SEPARATOR, "/")`, per the goal's *Path and the two
      legs* decision. `crates/mwl-stdlib/src/path.rs:104` (`join`), `:111` (`split`), `:118`
      (`normalize`).
- [ ] **`relativeTo` and `join` undo each other, and the boundary is where they stop** —
      `join($base, relativeTo($p, $base))` normalizes back to `$p` over a table, and the pair parts
      exactly where no relative path exists (a different drive, `isAbsolute` disagreeing between the
      two arguments). Both sides of that bound named together. `crates/mwl-stdlib/src/path.rs:104`
      (`join`), `:118` (`normalize`), `:125` (`isAbsolute`), `:132` (`relativeTo`).

## Backlog

- The 9 oracle gaps `gaps.py --differential` now names. `Core\Encoding::encodeText`/`decodeText`/
  `isValidText` against `mb_convert_encoding` and `Core\Time::fromIso`/`parse`/`at` look takeable;
  `now`/`monotonic`/`sleep` want `--ORACLE-DIVERGES--` and a reason, or nothing.
- `Core\Time\Instant` is 9 members over 0 cases naming it and `Core\Time\DateTime` 17 over 1 —
  the two thinnest classes on `python tools/gaps.py --coverage`, well below `Core\Path`.
- `basename`/`dirname`/`extension`/`withExtension` agree on where the name ends — read
  `path-decomposes-a-path-without-touching-the-disk.mwlt` first; it may already own the claim.
- The measured slice cap is now **4** where AGENTS.md § *Session workflow* step 2 says 2; sessions
  end at 114k mean against the 200k ceiling. Raising it is the largest remaining clock lever and the
  only one that can degrade a session, so it is the user's call, one step at a time, re-measured with
  `python tools/loop-stats.py` after each run.
- `Core\Json::decodeAs<T>`'s decoder reads scalar-fielded classes only — ADR 0071,
  `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `docs/adr/0097-development-server-and-proxied-origin.md:47` links `../plan/m13.md`, which that ADR
  itself deleted. Pre-existing; the link should go, not the sentence.
