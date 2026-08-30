# Handoff

## State

**Stage 5 is closed and stage 6 has its first refusals.** `python tools/bench.py --warm-start --max-ms
10` measures the CLI's start floor — a second `nvs run` of `benches/userland/00-baseline.nvs`, with the
binary and the script already in the OS page cache — and reports **6.7 ms** against the budget here. It
does not exercise `Cache::load` and says so on stdout: there is still no payload to load, per
`crates/nvs-cli/src/cache.rs` § *Known gaps*. It measures `target/release/nvs.exe` and **nothing builds
one** — the playbook's *Running things* bullet is what that costs a fresh machine.

**ADR 0073 §§ 1-3 is `crates/nvs-config/src/schedule.rs`**, run from `resolve()` after `app::bound`
because a scheduled script is checked against the `[capabilities] script.spawn` roots the merge
settled. Six refusals, every one `E0611`: no `name` or a duplicate one, no `cron` or one outside § 2's
five-field dialect, no `scope` or a third word for it, `fleet` with no shared store, no `script` or one
outside the roots. The roots check is `Capabilities::allows` and never a second
canonicalize-then-prefix, which is the standing decision in `docs/agent/loop-goal.md` applied.

**`scope = "fleet"` refuses every tree today.** Nothing spells ADR 0059's shared store — ADR 0073's own
*Scope* excludes it — so `schedule::configures_a_shared_store` answers `false` for every configuration.
That is § 3's rule applied, not a stand-in for it; whoever gives the store a block changes that one
function and nothing else in the module.

`tests/resolve.rs`'s `a_value_array_replaces_where_a_table_appends` now appends `[[extension]]` entries
rather than `[[schedule]]` stubs, which the new boot check refuses on sight.

Orientation gaps: `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed,
and names no `tools/` file, so this session's own item printed no map line; `[context] adrs` should drop
**0042 § 3** (landed) and carry **0073 §§ 1-3**, which was sliced by hand here.

## Next group

**ADR 0074's boot refusals, in the shape ADR 0073's just landed in.** One file set:
`crates/nvs-config/src/{tree.rs,schedule.rs,request.rs}` and `crates/nvs-config/tests/resolve.rs`.

- [ ] **ADR 0074's two** — `cors_star_origins_with_credentials_true_is_refused` and
      `same_site_none_without_secure_is_refused`, over the merged tree the way
      `crates/nvs-config/src/schedule.rs:67` is run from the resolver. Anchors:
      `crates/nvs-config/src/tree.rs:387`, `crates/nvs-config/src/tree.rs:405`,
      `crates/nvs-config/tests/resolve.rs:696`.
- [ ] **The clause that needs one implementation rather than two** —
      `the_same_two_refusals_come_from_config_set_as_from_the_boot`: `Core\Config::set` has to refuse
      what the boot refuses, out of the same predicate. Anchors: `crates/nvs-config/src/request.rs:92`,
      `crates/nvs-config/src/tree.rs:342`.
- [ ] **§ 6's remaining `[[schedule]]` keys** — `overlap` is `skip`, `queue` or `kill` and `timezone` is
      a zone that exists; both deserialize unchecked today. Anchors:
      `crates/nvs-config/src/schedule.rs:67`, `crates/nvs-config/tests/resolve.rs:748`.

## Backlog

- The shared store has no configuration block, so `scope = "fleet"` cannot boot — ADR 0073 § 3, ADR 0059.
- `Core\Secret::reveal()` is not in the `Core` registry — goal 3, item 18.
- `Live::admit`'s same-class check asks the answer, not the argument — `crates/nvs-runtime/src/graph.rs`.
- No cache payload until `nvs-codegen` gains a second `Module` — `crates/nvs-cli/src/cache.rs`.
- A release binary older than `crates/` still passes the warm-start check — playbook, *Running things*.
