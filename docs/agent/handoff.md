# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 7 are complete. **Stage 8's `nvs-suite` check is green** — all three cases are on
disk. What is left of stage 8 is its `cargo-named` half: a served request's mount origin, and the
boot refusal for a mount that needs one and has none. Nothing is blocked.

**`rule:routing/link-carries-the-mount-prefix` is `shipped`.** `nvs_stdlib::router`'s `mounted` reads
`Ctx::inbound`'s `mount_prefix` and joins it in front of `substitute`'s answer for all three of
`url`, `urlAbsolute` and `urlSigned` — one function, because a prefix joined at two of the three
sites is how they would come to disagree about where the module is mounted. A program carrying no
request and a request whose door stripped nothing are one answer there, so neither has a branch, and
a signed link survives a remount because `signed_payload` is the route's name and its `$params`.

**A synthetic request describes its door.** `Core\Test::request`'s bag carries `mount` beside
`headers` and `body`, defaulting to `""` and not `null` for the reason
`rule:routing/a-request-reads-its-mount` makes `mount()` never-`null`; `nvs_runtime::InboundSpec`
holds the prefix and the captures and hands both to `Inbound::set_mount` in `build`, while the bag
key is the prefix alone. Both `loop-goal.toml` copies also lost a floor name this goal had renamed
away — see the playbook's bullet, not this file.

## Next group

**Stage 8: the mount's origin, at boot and on a served request** — one file set:
`crates/nvs-config/src/mount.rs`, `crates/nvs-cli/src/serve.rs` and `crates/nvs-server/src/mount.rs`.

- [ ] **A served request receives its mount's resolved origin** —
      `crates/nvs-config/src/mount.rs:64`'s `Mounted::origin` is substituted at expansion, and
      `crates/nvs-server/src/mount.rs:334` says reading it onto the request context is still this
      slice's to do, so `Core\Router::urlAbsolute` throws under a mount that resolved one. The served
      carrier is built at `crates/nvs-cli/src/serve.rs:783`, beside the `carry` that already puts the
      prefix on it; an origin is a context field rather than a carrier one, written through
      `crates/nvs-runtime/src/ctx/wiring.rs:351`'s `set_origin`, whose command-line caller is
      `crates/nvs-cli/src/main.rs:2129`. `rule:routing/an-origin-is-per-mount-and-checked-at-boot`.
      Test: `a_served_request_receives_its_mounts_resolved_origin`, `-p nvs-cli`.
- [ ] **A mount whose unit calls `urlAbsolute` and resolves no origin refuses the boot** —
      `crates/nvs-config/src/mount.rs:27`'s module doc says the question needs the compiled unit and
      so belongs beside whatever compiles a mount's entry, which is the `expand` call at
      `crates/nvs-cli/src/serve.rs:352`. It runs per resolved mount and re-runs on reload, so the
      failure is at deploy time rather than in a sent message.
      `rule:routing/an-origin-is-per-mount-and-checked-at-boot`. Test:
      `a_mount_whose_unit_calls_url_absolute_and_resolves_no_origin_refuses_the_boot`, `-p nvs-cli`.

## Backlog

- Spec § 13's `Core\Test` cell states the roster in English, so `request`'s bag is spelled nowhere in
  the spec — `docs/agent/carried-gaps.md`.
- `scope = "fleet"` parses, boots and is not armed — `crates/nvs-server/src/schedule.rs`.
- `Core\Response::html` and `sendFile`, the two § 15 members still outstanding —
  `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`.
- `Core\Metrics`'s three rows belong to goal `m8-stdlib-depth`, not here — `docs/agent/loop-goal.md`
  § *Not this goal*.
