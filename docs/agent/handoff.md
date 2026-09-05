# Handoff

## State

**Goal 6, M7 — stage 9's `differential` check is green.** `tests/differential/` holds **275 passing
cases** against `min_passing = 275`, so the margin is zero: a case removed or newly skipped re-opens
the check. Nothing in the suite skips — PHP 8.5.9 is on `PATH` on this box, so every `--ORACLE--`
case runs.

**Stage 9 has one check left, and it is not a test to write.** The `tools/bench.py --serve-vs-fpm
--record benches/serve.json` `command` row names a flag `bench.py` does not have (its flag block is
`tools/bench.py:398-436`) and an artifact `benches/` does not hold (it holds `abi-probe/`,
`members/`, `userland/`). `bench.py` today is a *userland CLI* harness — nvs against php and bun over
`benches/userland`, `--json` appending one NDJSON record per case — so the serve-versus-FPM
comparison is new work, and it needs both a load generator and an FPM to compare against. **This box
has neither**: the PHP here is the CLI SAPI (`php -m` lists no `fpm`, and no `mbstring` or `intl`
either), and Windows has no php-fpm at all.

**`python tools/gaps.py`'s differential list is empty** — 0 members with a PHP twin and no oracle
case — so picking is by *claim* and not by member. The nineteen cases this session added are members
that were called somewhere in the corpus but had no case of their own, which is the ranking that
replaced the handoff's earlier "gaps.py ranks the twins with no oracle case".

**`orient.py` printed no spec section, and every differential case needs one**: the **Replaces**
column of `docs/spec/01-core-library.md` §§ 1-3 is what names a member's PHP twin, and reading it
cost this session three calls. The goal's `[context]` manifest has no `spec` field; adding one that
slices § 1, § 2 and § 3 would pay for itself in one session.

**Five migration members are still owed**, listed in
`crates/nvs-stdlib/tests/migration-members-outstanding.txt`. The list only shrinks.

## Next group

**`bench.py`'s serve-versus-FPM leg** — the last stage-9 check. File set: `tools/bench.py`,
`benches/`, `docs/plan/m7.md`.

- [ ] **Decide what the comparison is on a box with no FPM, and record it in the module docstring at
      `tools/bench.py:1`.** M7's *Verify* paragraph asks for `wrk`/`oha` throughput against PHP 8.5 +
      FPM + opcache, recorded in `benches/`; neither generator nor FPM is installed here. Which
      baseline stands in — `php -S`, `php-cgi`, or the leg refusing with a named reason — is a
      decided-and-recorded call, pre-authorized by `loop-goal.md` § *Standing decisions*.
- [ ] **Add `--serve-vs-fpm` and `--record PATH`** beside the existing flags at `tools/bench.py:398`,
      dispatched from `main` at `tools/bench.py:392`. `--record` writes one JSON document where
      `--json` at `tools/bench.py:436` appends NDJSON, so the two are separate outputs rather than
      one flag with a mode.
- [ ] **Drive `nvs serve` and write `benches/serve.json`** through the timing that already exists —
      `measure` at `tools/bench.py:262` and `warm_start` at `tools/bench.py:292` — rather than a
      second timing path, so the artifact is the check's own evidence.

## Backlog

- `Core\Str::fold` and `::normalize` can get no differential case on this box: their twins are
  `mb_convert_case` and `Normalizer`, and this PHP has neither extension — `docs/spec/01-core-library.md` § 1.
- `Core\Arr::from`, `::flatten`, `::mapKeys` and `::overlayDeep` still have no case of their own —
  spec § 2's *Structure* and *Combining* tables.
- `Core\Math::cbrt` diverges from `pow($n, 1/3)` over a negative argument (`NAN` there, the real root
  here) and nothing pins it — spec § 3.
- `tools/gaps.py --differential` now always prints an empty list; its `--help` still sells it as the
  way to pick a differential case — `tools/gaps.py`.
- Five migration members owed — `crates/nvs-stdlib/tests/migration-members-outstanding.txt`.
