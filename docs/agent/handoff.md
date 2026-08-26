# Handoff

## State

**Conformance is at 550 of 600 on disk and it is the only frontier left**, but two of those cases
are **uncommitted** — see *Next group*. Verify is green (1597 cargo tests, 74 suites, 550
conformance, 159 differential, clippy and fmt clean) and runs both `.mwlt` trees itself, so after a
green `verify.py` there is nothing else to run (playbook, *Running things*).

This session was **user-directed, not loop work**, and added no library code. It answered whether a
corrupt request or an engine panic can take the server down, and landed the answer as
[ADR 0106](../adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) — fourteen
sections covering the paths that go *around* 0002's containment, 0020's ladder, 0095's refusals and
0097's ceiling: `abort()`, a `SIGSEGV` from the engine's own recursion, a `SIGBUS`, an accept loop
that spins, and a core that is alive and never returns. It folds into 0002 (§ *Corollary*), 0020
(§§ 1 and 4), 0078 (the recompile wave) and 0097 (`max_in_flight`, the accept loop, the watchdog).

**The process boundary was examined and rejected**, with a named revisit trigger at
[0083](../adr/0083-persistent-connections-are-isolates.md); 0106 § *Alternatives rejected* holds the
three findings and § *Revisiting* the two numbers that must exist before anyone reopens it. Do not
re-derive that argument — it is written down.

The earlier note here about 103 uncommitted ADRs is stale: that by-hand pass is committed.

## Next group

**Two of the previous group's three slices are already written and untracked in the tree** —
`tests/conformance/core/uri-seven-readers-with-and-tostring-are-one-parse.mwlt` and
`uri-percent-coders-are-two-inverse-pairs-over-a-byte-sweep.mwlt`. They pass under `verify.py` and
they are why the on-disk count is 550 against the plan's 548. They were left by an earlier session
and this one did not touch them. The file set is `crates/mwl-stdlib/src/uri.rs` plus those two
cases; `docs/spec/01-core-library.md` § 7 owns the `Uri` rules.

- [ ] **Check the two untracked `Uri` cases against the goal and commit them**, then reconcile the
      plan's conformance count in the same wrap. Nothing else in the tree is uncommitted.
- [ ] **`parseQuery` and `buildQuery` are one bracket convention** — the standing decision in
      `loop-goal.md` puts PHP's `a[]=1`/`a[b]=c` nesting in `parseQuery` in full; assert the round
      trip with `Core\Json::encode` on the parsed side (playbook: an `array<mixed>`'s elements
      cannot be indexed past the first level). `crates/mwl-stdlib/src/uri.rs:389`, `:396`.
- [ ] **`Core\Uri::resolve` and `compareTo`** — RFC 3986 § 5 reference resolution and the normalized
      order. `crates/mwl-stdlib/src/uri.rs:499`, `:506`.

## Backlog

- ADR 0106's M6 half — iterative teardown, `try_reserve` on input-sized buffers, per-decoder depth
  limits, the deadline flag in `Ctx`'s hot line. 0106 §§ 3, 4, 5; guards named in its § *Verification*.
- ADR 0106's M7 half — worker-root `catch_unwind`, accept-loop backoff, the watchdog, admission
  arithmetic, the blocking-pool rule. 0106 §§ 2, 6, 7, 8, 13.
- `Core\Time\Instant`'s and `DateTime`'s depth rows read low only because their values are never
  spelled; check with `gaps.py --member` before writing to them. `docs/agent/playbook.md`.
- `Core\Json::decodeAs<T>` still reads a scalar-fielded class only — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row —
  `mwl_stdlib::hash`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
