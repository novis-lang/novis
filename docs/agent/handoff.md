# Handoff

## State

Milestone `dossier`, goal `config-directives-1-3` — 16 features, **12 landed**, 4 open. Each of
`directive:deferred.max_concurrent`, `directive:extension` and `directive:http` now has its Rust
census test in `crates/nvs-config/tests/directives.rs`, one example with a blessed `.out`, an attack
and an `about.md`; `python tools/dossier.py --group 'config:directives'` is the only scoreboard
worth reading. A directive owes **one** example and no bench — `tools/data/dossier-policy.toml`'s
table is what each kind owes.

`nvs.toml` gained a `[deferred]` block. `max_concurrent = 256` is the number
`nvs_runtime::Ctx::deferred_max_concurrent` already answers where nothing is written, so the line
changes nothing this checkout does and gives the page a value to print; the block's other half stays
unwritten, because `docs/examples/config/deferred-deadline/`'s frozen output is of a deadline
arriving as the built-in default. `[http]` needed no such block at all: its shipped defaults are the
closed ones, so `(nothing, so the built-in default)` is that page's lesson rather than a gap.
Nothing is blocked.

## Next group

**Goal `config-directives-1-3`, items 13–16 — one file set:** `crates/nvs-config/tests/directives.rs`
(`keys_in` lists a block's accepted keys and `governing` resolves a row; `DIRECTIVES.iter()` is how
the `http` case reads its exceptions out of the registry rather than listing them), `nvs.toml`,
`docs/examples/config/<dir>/` and `tests/hostile/config/<dir>/`. The directory name keeps a key's
underscores and turns only its dots into dashes, so ask `python tools/dossier.py --id '<feature>'`
rather than deriving it. All four are `[http.client]`, so one `nvs.toml` block and one reading of
`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned` serves the first two.

- [ ] **`directive:http.client.pool_idle`** — `System` and `Reload`, and the census wants the ground
      rather than the class: the cap bounds a **core's** memory and not a request's, which is why it
      is the exception inside a block whose blanket row answers `Runtime`.
      `crates/nvs-config/src/directive.rs:133`,
      `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`.
- [ ] **`directive:http.client.pool_idle_timeout`** — `System` and `Reload`, the same row's other
      half: how long an idle pooled connection is kept before it is dropped.
      `crates/nvs-config/src/directive.rs:134`. The pair is already asserted as a set in
      `crates/nvs-config/tests/directives.rs:1045`, so this row's census wants what a *shortened*
      timeout spends — a reconnect and a TLS handshake per call — not the set again.
- [ ] **`directive:http.client.proxy`** — `System` and `Reload`, and the reason is not the pool's:
      where every outbound byte goes is the deployment's decision, and a per-call spelling would be
      a per-call way to narrow `rule:security/net-address-policy`.
      `crates/nvs-config/src/directive.rs:146`,
      `rule:http-server/an-outbound-proxy-is-operator-configured`.
- [ ] **`directive:http.client.socket`** — `Runtime` and `Reload`, the one row under `[http.client]`
      that is **not** an exception. `crates/nvs-config/tests/directives.rs:1166` already pins both
      bounds and the class, and it carries no `covers:` marker — adding
      `// covers: directive:http.client.socket` above its `#[test]` is the whole test half, so this
      item is an example, an attack and an `about.md`.
      `crates/nvs-config/src/directive.rs:154`,
      `rule:http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap`.

## Backlog

- Goal `config-directives-2-3` and `-3-3` are the rest of the registry — `docs/agent/goals/dossier/`.
- `Core\Program::id()` is a machine fingerprint, so an example may print its length and never its
  value — `crates/nvs-config/src/cache.rs`'s `build_stamp` owns why.
