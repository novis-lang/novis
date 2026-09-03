# Handoff

## State

**Goal 6, Stage 3: § 5's `[server]` block now bounds the request path as well as routing it.**
`max_in_flight` is ADR 0106 § 13's arithmetic rather than the number the file wrote —
`crates/nvs-server/src/admit.rs` is the whole of it — and the refusal is taken *before* the
handler is asked for a `Reply`, so a request over the ceiling selected no mount, allocated no
isolate and ran no Novis code. That module's own docs own the order, the relaxed process-wide
counter and why `ENGINE_RESERVE` is a stated reserve rather than a measurement (M7's *Verify* is
what replaces it with a number).

**The three inputs are configuration's.** `nvs_config::server::capacity_for` reads
`[server] max_in_flight`, the per-request cap — `[limits.hard] memory` where a ceiling is written,
otherwise `[limits] memory` — and `memory_budget()`, which **prefers a container's cgroup limit to
the host's memory**; that function's doc comment owns why. Either half absent leaves the written
ceiling standing, which is § 13's inert case and not a zero. `E0622` refuses a written `0`, the one
magnitude that is a refusal rather than a clamp.

**The four waits are asserted where the check looks.** `crates/nvs-server/src/io.rs`'s
`the_four_idle_timeouts_are_finite` sweeps every `Phase` under four *distinct* waits, because two of
§ 5's defaults are equal and a phase wired to the wrong field reads as correct against them alone.
The boot refusal of `false`/`0` stays `nvs_config::server`'s `E0619`.

**Unchanged limits.** No accept backoff on descriptor exhaustion (ADR 0106 § 8) and no wedged-core
watchdog (§ 7); a served request's context still carries no configuration snapshot
(`crates/nvs-server/src/serve.rs:417`); a Unix-domain `listen` entry still classifies and is then
refused in the CLI. The driver's `native examples/upload.nvs` failure is stage 5's frozen `want`
ahead of the frontier — the playbook's bullet on that owns it. `[context] adrs` gained `0106 §13`
this session: the item named it as its specification and the pack did not print it.

## Next group

**What is left of the `[server]` block, plus § 10's mount policy.** One file set:
`crates/nvs-config/src/tree.rs`, `crates/nvs-server/src/serve.rs`, `crates/nvs-server/src/admit.rs`.

- [ ] **A mount carries no policy of its own.** ADR 0097 § 10 — a mount routes and nothing else, so
      a `[[server.mount]]` block that grows a limit has re-implemented the per-app block goal 3
      built. The block is `crates/nvs-config/src/tree.rs:797`, and the assertion belongs beside the
      table it is about. Pins `a_mount_carries_no_policy_of_its_own`.
- [ ] **`isDraining` answers during a graceful shutdown.** ADR 0097 § 5's `health_path` — `200`
      while accepting, `503` while draining, empty body, skipped by the access log, and **no
      dependency checks**. The directive is `crates/nvs-config/src/tree.rs:773` and the state is the
      accept loop's `keep_serving` at `crates/nvs-server/src/serve.rs:381`, whose tail already parks
      on the outstanding tally that a drain has to wait out. Pins
      `is_draining_answers_during_a_graceful_shutdown`.
- [ ] **The accept loop backs off on descriptor exhaustion.** ADR 0106 § 8 as § 5 names it: an
      `accept` failing `EMFILE`/`ENFILE` returns immediately and fails again immediately, which is a
      fully utilised core and a log written at the speed of the loop. One log per window, not per
      attempt. `crates/nvs-server/src/serve.rs:381` is the loop and
      `crates/nvs-server/src/admit.rs` is where a process-wide bound already lives.

## Backlog

- `ENGINE_RESERVE` is 128 MiB by assertion; M7's *Verify* benchmark is what measures it —
  `crates/nvs-server/src/admit.rs`.
- A served request's context carries no configuration snapshot — `crates/nvs-server/src/serve.rs`.
- A Unix-domain `listen` entry classifies and is then refused — `crates/nvs-cli/src/serve.rs`.
- `orient.py` prints no map line for `crates/nvs-cli/src/*` — `[context] modules` in
  `docs/agent/loop-goal.toml` wants an `nvs-cli` pattern.
- `[server] trusted_proxies` and § 6's forwarded-header walk — ADR 0097 § 6.
