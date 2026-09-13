# Handoff

## State

**Goal `process-cache` — stage 7 is landed, and both of its checks are green.** A user's own secret
lives in the session through `Core\Session::setSecret` and `::getSecret`
(`rule:http-server/a-session-holds-a-secret-only-sealed`), sealed under a key ring and stored as
ciphertext in the record. The two halves of the construction are `sealed_into` and `opened_from` in
`crates/nvs-stdlib/src/session.rs`, and the record entry lives in the same sealed key space a cache
entry does, so `get` answers `null` for it.

**One construction, two domain octets.** `crate::cache`'s `bound` now takes the door's octet as an
argument — `cache::SEAL_DOMAIN` is `1`, `session::SEAL_DOMAIN` is `2` — and `sealed_key`,
`application` and `bound` are `pub(crate)` for the session to reach. That single octet is the whole
of why a value sealed for a cache does not open as a session secret, which
`a_value_sealed_for_the_cache_does_not_open_as_a_session_secret` asserts in both directions.

**The additional data is the domain octet, the application and the key, and never the session id**,
so a secret survives `regenerate`. There is no `ttl` and nothing is sealed in beside the value: a
session value lives as long as its session (`rule:http-server/session-expiry-belongs-to-the-store`).

**`Core\Session::set` now refuses a `secret` and names `setSecret`** —
`reject_secret_session_argument` in `crates/nvs-types/src/expr/quals.rs`, the fifth carrier of the
one graph-copy refusal and the second with a door of its own.

**The `[context]` gap is unchanged:** the goal's own stage prose (`docs/agent/loop-goal.md:118-137`)
is reachable from no field, so a session meets a stage only through the header the driver prints
above a failing check.

## Next group

**Stage 8: the rulebook** — one file set: `docs/rules/concurrency.json`, then one render.

- [ ] **`concurrency/a-secret-is-cached-only-sealed` is `shipped`** — its `"status"` field at
      `docs/rules/concurrency.json:394`, whose work landed in stage 5: `putSecret`/`getSecret` are
      on disk with their conformance cases, so `designed` is stale rather than a claim. Check the
      entry's `guardedBy` names the cases that now guard it before flipping it.
- [ ] **`concurrency/a-secret-fill-runs-once-per-process` is `shipped`** — the same field at
      `docs/rules/concurrency.json:413`, whose work landed in stage 6 — `elected`, `waited` and
      `filled` in `crates/nvs-stdlib/src/cache.rs` — with the same `guardedBy` check.
- [ ] **Render, then the goal is met** — `python tools/rules.py --render` rewrites
      `docs/rules/concurrency.md` and `docs/ground-rules.md`, which are generated and never edited;
      the stage's own four checks are at `docs/agent/loop-goal.toml:9849`, each a `rules.py --show
      <id>` wanting `shipped`, and the other two ids are already there. So this closes the goal:
      run `python tools/verify.py --doc`, fix every link it names, and write `DONE`.

## Backlog

- The website's rule mirror (`website/src/content/docs/docs/rules/`) is written by
  `website/scripts/sync-rules.mjs` and was not re-run here; nothing in `verify.py` reads it.
- A fleet-wide single fill, a `secret bytes` value, and the `nvs/rest` package are all named as not
  this goal's in `docs/agent/loop-goal.md` § *Standing decisions*.
