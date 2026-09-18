# Handoff

## State

Milestone `dossier`, goal `config-directives-1-3` — 16 features, **6 landed**, 10 open. Each of
`directive:cache.shared`, `directive:capabilities` and `directive:control.socket` now has its Rust
census test in `crates/nvs-config/tests/directives.rs`, one example with a blessed `.out`, an attack
and an `about.md`; `python tools/dossier.py --group 'config:directives'` is the only scoreboard
worth reading. A directive owes **one** example and no bench — `tools/data/dossier-policy.toml`'s
table is what each kind owes.

`nvs.toml` grew two more blocks that exist to be *read* rather than to let a fixture run: an
`[[app]]` block for the capabilities example, whose grants are booleans because a list grant is
invisible from inside a program (playbook, *Writing a test case*), and `[control] socket = false`,
which is what an absent `[control]` block already meant — `nvs_config::control`'s `Address::of`
reads both to `Disabled`, so nothing this repository does changed. Nothing is blocked.

## Next group

**Goal `config-directives-1-3`, items 7–9 — one file set:** `crates/nvs-config/tests/directives.rs`
(every directive census test lives here; `keys_in` lists a block's accepted keys and `governing`
resolves a row), `nvs.toml`, `docs/examples/config/<key-with-dots-as-dashes>/` and
`tests/hostile/config/<same>/`. Read `docs/examples/README.md` and `tests/hostile/README.md` once at
the start — they own what an example and an attack *are*, the pack prints neither, and there is no
`[context]` field that would.

- [ ] **`directive:debug.keep_temporary`** — `System` and `Reload`, the operator's alone because a
      request that could set it would exempt its own files from the sweep,
      `rule:core-classes/temporary-dir-sweep`. `crates/nvs-config/src/directive.rs:219`. Its sibling
      `io.temp_root` is `Boot` on the row above, which is the contrast the census test wants.
- [ ] **`directive:debug.inline`** — `RuntimeTighten` and `Reload`, one direction only: a request may
      turn its own inline dumps off and never on, `rule:errors/debug-dump`.
      `crates/nvs-config/src/directive.rs:227`. Note before writing the example: a `RuntimeTighten`
      row that is not a *quantity* cannot be set at all, so both directions come back `false` —
      `nvs_config::request`'s module doc owns why, and `[capabilities]`'s page already says it in a
      reader's words.
- [ ] **`directive:deferred.deadline`** — `Runtime` and `Reload`, and the first row in this goal a
      request may actually set: the cap is a host-sizing decision and the deadline is an ordinary
      per-request default, `rule:concurrency/deferred-is-bounded-by-two-directives`.
      `crates/nvs-config/src/directive.rs:246`. The example can show a `set` that *succeeds* and one
      the `[limits.hard]`-style ceiling refuses, which no page in this goal has shown yet.

## Backlog

- `Core\Config::get` answers nothing for every list-valued key, so a scoped grant and an absent one
  are one answer from inside a program — designed, but said nowhere a `Core\Config` reader looks
  (`crates/nvs-stdlib/src/config.rs`).
- The goal's remaining ten features, `directive:extension` onwards — `python tools/dossier.py
  --group 'config:directives'`.
