# Handoff

## State

**Goal `m7-server-surface` is complete, stage 13 included.** Stages 3 to 12 built the surface and
stage 13 is the rulebook: every rule the goal names now reports `shipped`, each carrying `guardedBy`
paths to the files whose tests fail when it breaks. `python tools/rules.py --check` and
`--render --check` are clean, and `python tools/verify.py` is green on all twelve legs — including
`doc`, which says every intra-doc link resolves. Nothing is blocked.

**Eight rules were flipped this session; three of the stage's eleven were already `shipped`** — the
enum-capture spelling, the fleet lease and the per-mount origin, which earlier stages had already
landed. The eight are `config/one-local-control-socket` and
`config/ctl-config-reports-the-live-snapshot`, `packaging/a-service-is-one-stored-argv` and
`packaging/a-service-answers-its-manager`, `http-server/a-unix-socket-listener`,
`observability/the-exporters-are-crates`, `observability/four-kinds-become-a-span` and
`observability/an-exporter-brings-no-second-scheduler-and-no-second-client`. Each one's guards were
read before the flip rather than assumed: the control endpoint's three operations at
`crates/nvs-server/src/control.rs:593`, the client half at `crates/nvs-cli/src/ctl.rs:814`, the
separator overload at `crates/nvs-config/src/server.rs:912`, the Unix peer's implicit forwarded trust
at `crates/nvs-server/src/forwarded.rs:587`, the four span kinds at `crates/nvs-server/src/trace.rs:607`
and the push at `crates/nvs-server/src/otlp.rs:944`.

**`verify.py` runs the doc leg now**, so the backlog line claiming it does not is gone.

## Next group

**Nothing open in this goal.** The chain's next entry is goal `m8-db-queue`, which
`python tools/chain.py` installs once this one retires; its own file and `[context]` manifest carry
the group, and this handoff is overwritten by the goal switch.

## Backlog

- `Core\Metrics`'s three rows — goal `m8-stdlib-depth`.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30`, `crates/nvs-server/src/bounds.rs:62`,
  `crates/nvs-types/src/response.rs:29` and `crates/nvs-stdlib/src/cli.rs:130` — goal
  `unowned-closures`.
