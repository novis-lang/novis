# Handoff

## State

**Goal 6, M7 — stage 9's last check is green.** `python tools/bench.py --serve-vs-fpm --record
benches/serve.json` prints `requests/sec` and `recorded` in about 2.3 s, so the `command` row at
`docs/agent/loop-goal.toml:4063` is satisfied. Stage 9 is the last of the ten stages and the whole
goal is 250 checks; whether anything *else* is open is the driver's next acceptance run to say, not
this session's.

**On this box: `nvs serve` 29,518 requests/sec against `php-cgi`'s 10,620 — 2.78x, at concurrency
1.** The decision behind those numbers is recorded in `tools/bench.py`'s module doc, § *The
serve-versus-FPM leg*, which is its only home: the generator is that file rather than `wrk` because
no generator is installed here; the peer is `php-cgi -b` driven over FastCGI with opcache, which is
not a stand-in for FPM but **the same SAPI**, and it pays no HTTP parse and no proxy hop while `nvs
serve` pays both, so the comparison is biased towards PHP; `php -S` and no-baseline-at-all are
fallbacks, reachable on a box that has the strong peer through `--serve-baseline
{fcgi,builtin,none}` so that neither branch is code nobody has run. Concurrency defaults to 1
because neither Windows peer answers a second connection at all — at `--concurrency 4` the FastCGI
peer simply never replies, and the leg now says so by name instead of timing out silently.

**`benches/serve.json` is a JSON array capped at 100 runs**, and the driver's acceptance check
appends one every iteration. A session will therefore find it dirty without having written it:
committing the newer measurement or checking it out are both fine, and neither is a regression.

**Five migration members are still owed**, listed in
`crates/nvs-stdlib/tests/migration-members-outstanding.txt`. The list only shrinks.

**`orient.py` still prints no spec section**, and the `[context]` manifest at
`docs/agent/loop-goal.toml:74` still has no `spec` field — carried from the previous handoff because
it is still true and still costs three calls a session that writes a differential case.

## Next group

**Closing the goal, and the two docs this leg leaves behind.** File set:
`docs/agent/loop-goal.toml`, `docs/plan/m7.md`.

- [ ] **Take whatever the driver's ledger names, and if it names nothing, write `DONE`.** The check
      this session closed is `docs/agent/loop-goal.toml:4063`; stage 9 is the last stage, so a clean
      acceptance run means the goal in `docs/agent/loop-goal.md` is met.
- [ ] **Reconcile M7's *Verify* sentence with what was measured** — `docs/plan/m7.md:105` still asks
      for `wrk`/`oha` against PHP-FPM. Say what stands in and point at `tools/bench.py`'s § *The
      serve-versus-FPM leg* rather than restating it; `python tools/plan.py --amend M7` is the tool,
      and whether it takes a `M7:verify` selector is the first thing to find out.
- [ ] **Add a `spec` field to the goal's `[context]`** — `docs/agent/loop-goal.toml:74` — slicing
      `docs/spec/01-core-library.md` §§ 1-3, whose *Replaces* column names a member's PHP twin. Two
      sessions have now paid three calls each to read it by hand.

## Backlog

- A second serve case beyond `hello`: today's figure is request overhead only — `benches/serve/`.
- `Core\Db\Queryable::stream` and four more — `crates/nvs-stdlib/tests/migration-members-outstanding.txt`.
- `--concurrency` past 1 needs a box with a real FPM pool — `tools/bench.py` § *The serve-versus-FPM leg*.
