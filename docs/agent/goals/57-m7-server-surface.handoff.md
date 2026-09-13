# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run — has just started; nothing of it has landed yet.** Goal `m4b-editor`'s whole list is this goal's Stage 1 floor.

**Every design call the stages reach is already decided, in the goal's § *Standing decisions*.** The
one record that states the calls no current rule holds is not written yet; Stage 2 writes it. Do not
re-decide these:
- an enum case is spelled by its backing value, or else by its case name;
- an exporter brings no second scheduler and no second client;
- `Core\Request::bytes()` exists, and `body()` refuses an ill-formed payload rather than repairing it;
- the control endpoint runs on one thread of its own;
- `nvs service` is tested through a recording `Manager` and never against a real one.

## Next group

**Stage 2: the record** — one file set: `docs/decisions/`, `docs/rules/routing/`,
`docs/rules/observability/`, `docs/spec/01-core-library.md`.

- [ ] **The record** — the next free number in `docs/decisions/`, `changes.creates` the two rules the
      goal's stage 2 table names, `changes.modifies` `routing/a-capture-narrows-to-a-closed-set`
      (`docs/rules/routing/a-capture-narrows-to-a-closed-set.md:17-19`) and
      `observability/the-exporters-are-crates` (`docs/rules/observability/the-exporters-are-crates.md:1-4`).
      Before writing the exporter rule, read `Cargo.lock` and each candidate crate's own manifest for
      what `metrics-exporter-prometheus` and `opentelemetry-otlp` pull in — *not checked* while the
      goal was written. The body also names the drain's bound (Stage 3) and `sendFile`'s signature.
- [ ] **The two rule fragments and their JSON entries**, both `designed`; the two amendments; and spec
      § 15's `sendFile(…)` at `docs/spec/01-core-library.md:1122` spelled `sendFile(string $path)`.
      Then `python tools/rules.py --render`.

## Backlog

- Stage 3 — the control socket, `nvs ctl` and the drain. Files: `crates/nvs-server/src/control.rs:26`,
  `crates/nvs-config/src/control.rs:241`, `crates/nvs-cli/src/serve.rs:824`, `crates/nvs-cli/src/main.rs:745`.
  This is the keystone: stages 5 and 12 need it.
- Stage 4, the Unix listener (`crates/nvs-host/src/net.rs`), gets its own session. So does stage 5,
  `nvs service` (`crates/nvs-cli/src/service.rs:140`). After stage 5, install for real by hand on each
  platform and write the result down; no check does that.
- Stages 6 and 7 are the response body and `echo` (`crates/nvs-stdlib/src/response.rs:261`,
  `crates/nvs-runtime/src/ctx/output.rs:455`) and the request input with `Test::request`
  (`crates/nvs-stdlib/src/request.rs:2582`, `crates/nvs-stdlib/src/test.rs:412`). The two share
  `nvs-stdlib`'s registry.
- Stages 8 and 9 are the routes (`crates/nvs-types/src/routes.rs:1787`, `crates/nvs-stdlib/src/router.rs:825`)
  and the schedule (`crates/nvs-server/src/schedule.rs:301`, `crates/nvs-stdlib/src/cache/redis.rs:162`).
  Stage 9 needs the compose file's `redis` running.
- Stages 10 and 11 are the metrics export and the spans. Files: `crates/nvs-server/src/metrics.rs:388`,
  `crates/nvs-server/src/route.rs:41`, `crates/nvs-runtime/src/trace_context.rs:22`.
- Stage 12 (the served path end to end, `crates/nvs-cli/src/serve.rs:1570`), then stage 13's flips.
- When this goal's last check goes green the driver takes goal `m8-db-queue`.
