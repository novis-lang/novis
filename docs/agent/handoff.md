# Handoff

## State

**Goal 6, M7 — stage 7's `nvs-server (hot reload)` check was misfiled and is now green.** All
three names moved off `-p nvs-server`: `docs/agent/loop-goal.toml:3912` is two blocks now,
`nvs-cli (hot reload)` for the two cache tests and `nvs-config (the validate default)` for the
third, and `docs/agent/goals/6-server.toml` is the byte-identical copy the next `goal-switch.py`
would otherwise restore over it. `crates/nvs-server` resolves through `nvs-cli`'s compiler and
owns no cache, which is why neither claim had a spelling in that crate.

**ADR 0091 § 3a's third row landed as behaviour, not only as a test.**
`nvs_config::cache::Revalidation::from_config` now takes `validate`'s startup default from
`[mode] default` — `never` in production, `mtime` in development, and a tree that writes no mode
at all is production per § 5. `revalidate_freq` stays mode-independent, which § 3a states in its
own words. The tradeoff, since it changes an unconfigured host: a tree with no `nvs.toml` no
longer `stat`s a path it has compiled, which is strictly fewer syscalls on the request path
(priority 3) and is what the row asks for; a developer's tree writes `mode = "development"`, and
§ 3's first property keeps `[opcache] validate` overriding the mode either way.
`Revalidation::default()` is now the type's own value and reached only for the cap — its doc says
so.

`tools/orient.py` and `tools/peek.py` sliced `§3a` as § 3; both are fixed and the playbook bullet
above owns why it was invisible.

## Next group

**The control endpoint's other half — nothing accepts on it, and there is no client.** ADR 0078
§§ 3, 5 and 6, whose *Decision* the standing decisions already close: `reload` is the socket's
only operation and there is no network-reachable control surface. The file set is
`crates/nvs-cli/src/serve.rs`, `crates/nvs-cli/src/main.rs` and `crates/nvs-server/src/control.rs`
— but **the driver's own next acceptance failure outranks this list**, and it has not been seen
yet, because the three checks above were the ones holding it.

- [ ] **`nvs serve` creates the control endpoint and accepts on it** — ADR 0078 § 3, over
      `crates/nvs-cli/src/serve.rs:274`, where the request listener is bound, and
      `crates/nvs-server/src/control.rs:280`, which is the handler with no caller. The address is
      `nvs_config::control::Endpoint` at `crates/nvs-config/src/control.rs:137`; the 0600 creation
      and its refusals are landed and tested, so what is missing is only the accept loop beside
      the request one. `crates/nvs-cli/src/serve.rs:496` already names the socket in a comment.
- [ ] **`nvs ctl reload` — the client** — ADR 0078 § 6. There is no `Ctl` variant in the
      subcommand enum at all: `crates/nvs-cli/src/main.rs:320` is the `Service` variant, whose
      doc comment calls `nvs ctl` its own precedent, so the namespace is described and unbuilt.
      One `POST /reload` over the local endpoint, and the report
      `nvs_config::control::reload` at `crates/nvs-config/src/control.rs:282` already computes —
      including the changed `Boot` key it names rather than ignores — printed for an operator.
- [ ] **The refusals a client makes reachable** — `crates/nvs-server/src/control.rs:38`'s
      "names no operation" answer has only a synthetic caller today; assert it over a real
      connection once one exists, beside § 6's rule that nothing sent here reaches a program.

## Backlog

- The queue suite leaves claimable jobs in the shared `novis_test`: `clear` at
  `crates/nvs-stdlib/tests/queue.rs:395` runs at the head of a case and never at the end, and the
  roster in `crates/nvs-cli/src/worker.rs:239` claims from every queue. Nineteen call sites, each
  ending with a different connection state.
- Registration itself — `install`/`uninstall`/`start`/`stop`/`status`/`run`, the SCM call and the
  systemd write. `crates/nvs-cli/src/service.rs`'s module doc § *What is on disk, and what is not*.
- ADR 0091 § 3a's other two rows — `[server] dispatch` and `[server] static` — are documented as
  mode-selected at `crates/nvs-config/src/tree.rs:773` and are still not chosen by the mode.
  `Validate::started_in` is the shape the two of them want.
- `--fault-inject` is matched by word in `service.rs`; if `nvs run` ever gains a second hook the
  allowlist needs the argument's own grammar.
