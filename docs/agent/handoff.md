# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is goal
`m4b-editor`'s list carried in as the floor. **Stage 2 is done:** [0186](../decisions/0186.md) is on
disk, it creates `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name` and
`rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client` (both `designed`),
amends `rule:routing/a-capture-narrows-to-a-closed-set` and
`rule:observability/the-exporters-are-crates`, and `docs/spec/01-core-library.md` § 15 now spells
`sendFile(string $path)`.

**Nothing of stages 3-13 has landed.** The goal's § *Standing decisions* is still the pre-authorized
list; 0186 is the only ADR number this goal opens, and it is now spent.

**What 0186 settled, so it is not re-derived:** an enum capture matches a **written** backing value
and a **case name** where the value was counted; neither `metrics-exporter-prometheus` 0.18.3 nor
`opentelemetry-otlp` 0.32.0 is takeable, so both wire formats are written here and a crate may only
be an encoder; the drain's bound is a new `[server] drain_timeout`, `"30s"` with nothing configured,
and the drain **wakes** parked waits rather than letting them expire.

## Next group

**Stage 3: the control socket, `nvs ctl`, and the drain** — one file set:
`crates/nvs-config/src/control.rs`, `crates/nvs-config/src/server.rs`,
`crates/nvs-config/src/default.toml`, `crates/nvs-server/src/control.rs`,
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/main.rs`, and a new `crates/nvs-cli/src/ctl.rs`.

- [ ] **`[server] drain_timeout`, and the drain that wakes a parked wait** — the directive in
      `crates/nvs-config/src/server.rs` and its comment beside the four waits at
      `crates/nvs-config/src/default.toml:411-416`; `"30s"` with nothing configured, `Boot`-class
      with the block (`rule:http-server/the-server-block-is-boot-class`), never unbounded
      (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`). The period is read where
      `crates/nvs-runtime/src/drain.rs:52` holds the bit, and the close stays the connection's own
      (`rule:concurrency/a-drain-closes-a-connection-cleanly`, [0186](../decisions/0186.md) § 3).
- [ ] **The control endpoint's three operations** — `Operation`
      (`crates/nvs-server/src/control.rs:26`) gains `GET /config` and `GET /status` beside
      `POST /reload`, which publishes through `crates/nvs-config/src/control.rs:276` with the unit
      cache's count as `held`; `nvs serve` binds `[control] socket` at
      `crates/nvs-config/src/control.rs:241` before any listener accepts, and refuses the boot on a
      refusal (`rule:config/one-local-control-socket`,
      `rule:config/ctl-config-reports-the-live-snapshot`,
      `rule:config/a-reload-names-what-it-could-not-apply`). Every answer carries the version header.
- [ ] **`nvs ctl`, the signal handler and `sd_notify`** — a new `crates/nvs-cli/src/ctl.rs` wired from
      `crates/nvs-cli/src/main.rs`, refusing an answer from another version and naming both;
      `crates/nvs-cli/src/serve.rs:79` installs the terminating-signal handler whose whole effect is
      `Drain::process().begin()` (`crates/nvs-stdlib/src/signal.rs:42-44`), and sends `READY=1`,
      `RELOADING=1` and `STOPPING=1` by hand over the `NOTIFY_SOCKET` datagram, no crate
      (`rule:config/no-network-control-surface` bounds what the endpoint may be).

## Backlog

- `[metrics] endpoint` and `[trace] endpoint` are commented as `:4317`, OTLP's gRPC port, at
  `crates/nvs-config/src/default.toml:727-740`; the push is OTLP/HTTP on `4318` — stage 11's.
- `E0819` is claimed by [0186](../decisions/0186.md) § *Diagnostics* and not yet defined in
  `crates/nvs-diagnostics/src/lib.rs` — stage 8 defines it with the enum-capture match.
- Whether the OTLP encoder is `prost` or hand-written is stage 11's call under
  `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`; `prost` is in
  neither `Cargo.lock` nor the registry cache today.
- `§15 Response::sendFile` and `§15 Response::html` still sit in
  `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt:30-31` — stage 6 strikes both.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` belong to goal
  `unowned-closures`, not here.
