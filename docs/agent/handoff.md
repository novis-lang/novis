# Handoff

## State

**Stage 6's closure test is green and ADR 0074's refusals have landed.**
`every_capability_bearing_member_declares_its_capability` is in
`crates/nvs-stdlib/tests/capability.rs` and its frozen allowlist, `NEEDS_NO_CAPABILITY`, is
**empty** — every member of every capability-bearing class on disk declares what it needs, which is
the strongest state ADR 0118 § 7 describes. The one capability-bearing class today is `Core\File`
and both its members are in `registry::CAPABILITIES`. The test asserts that positively, so it cannot
pass by finding nothing to check.

**ADR 0074 §§ 2-3's two refusals are `crates/nvs-config/src/http.rs`**, run from `resolve()` after
`schedule::validate` because which `origins` and `secure` are in force is a question only the merged
stream has answered. Both are `E0612`. `Inbound` holds the four values the two questions are decided
from and `Inbound::meaningless` is the only place the condition is written; the boot reads those
values off the typed tree and `Request::stays_meaningful` reads them off the snapshot, the overlay
and the proposed assignment. That is the clause needing one implementation rather than two, and
`the_same_two_refusals_come_from_config_set_as_from_the_boot` holds it by asserting the two
mechanisms **agree** over six moves rather than asserting what either answered.

**An absent boolean is its shipped default, not `false`.** § 3 ships `secure = true`, so a tree
writing `same_site = "None"` and nothing else boots; reading absence as `false` would refuse a
correct configuration, which is the one direction this check must not fail in.

Orientation gaps, unchanged from the last session and still worth fixing: `[context] modules` names
`crates/nvs-host/src/budget.rs`, which never existed, and names no `tools/` file. `[context] adrs`
should drop **0042 § 3** (landed) and carry **0074 §§ 2-3** and **0118 § 7**, both sliced by hand
here.

## Next group

**Stage 6's adversarial suite — item 18, and the last `-p nvs-config` check open.** One file set:
`crates/nvs-config/tests/request.rs` and `crates/nvs-config/tests/capability.rs` (new), over
`crates/nvs-config/src/{request.rs,capability.rs}`.

- [ ] **`a_script_attempting_to_set_a_system_directive_fails`** — m6.md's *Verify*, over the
      `Request` harness that already holds the ceiling cases. `a_system_directive_is_not_settable_by_a_request`
      at `crates/nvs-config/tests/request.rs:159` is the near neighbour; what this adds is the
      adversarial half — every `System` row refused, asserted by counting over the registry rather
      than on one key. Anchors: `crates/nvs-config/tests/request.rs:159`,
      `crates/nvs-config/src/request.rs:146`.
- [ ] **`a_script_attempting_to_widen_a_capability_fails`** — ADR 0005's `RuntimeTighten` half, which
      `Request::set` already refuses for a reason the module doc's fourth paragraph owns; the case
      pins that a grant cannot be widened and says which of the two reasons refused it. Anchors:
      `crates/nvs-config/src/request.rs:146`, `crates/nvs-config/src/capability.rs:1`.
- [ ] **`spawn_script_without_the_capability_fails`** — the check names `-p nvs-config`, so it is
      `Capabilities::allows(Cap::ScriptSpawn, …)` answering `false` for an ungranted tree, beside the
      roots check `schedule.rs` already runs. Anchors: `crates/nvs-config/src/schedule.rs:67`,
      `crates/nvs-config/src/capability.rs:1`.

## Backlog

- § 6's remaining `[[schedule]]` keys — `overlap` is `skip`/`queue`/`kill`, `timezone` is a zone name
  — ADR 0073 § 6, in `crates/nvs-config/src/schedule.rs`.
- Stage 8's conformance floor is 1050 and the tree is at 1011; the `[[check]]` names eight cases
  under `tests/conformance/{config,cap,isolate,error}/` — `docs/agent/loop-goal.toml`.
- Stage 7's bundler, ADR 0048 — untouched, and last on purpose.
- `crates/nvs-cli/src/cache.rs` § *Known gaps* still decides a payload, so `Cache::load` is unexercised.
- `[context]` in `docs/agent/loop-goal.toml` needs the three fixes § *State* names.
