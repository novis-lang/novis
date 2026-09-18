# Handoff

## State

Milestone `dossier`, goal `config-directives-1-3` — **all 16 features landed**, and
`python tools/dossier.py --verify --only …` over the goal's whole list reports nothing owed,
0 failed examples and 0 failed hostile cases. The four `[http.client]` rows of this session each
carry a census test in `crates/nvs-config/tests/directives.rs`, one example with a blessed `.out`,
one attack and an `about.md`. `python tools/verify.py` is 13 of 13 green.

`nvs.toml` gained an `[http.client]` block writing `pool_idle = 16` and `pool_idle_timeout = "30s"`
— the numbers `nvs_stdlib::http`'s `DEFAULT_POOL_IDLE` and `DEFAULT_POOL_IDLE_TIMEOUT` already apply
where nothing is written, so this checkout holds the same connections with the lines as without
them, and the two pages print a value instead of `(nothing)`. The block's other halves stay
unwritten on purpose, each for its own reason, and the `nvs.toml` comment is where that is recorded:
`[http.client.socket]`'s page is the one that shows a bound arriving as the built-in default and
then being changed from code, and an absent `[http.client.proxy]` **is** how a deployment says it
wants no proxy (`rule:http-server/an-outbound-proxy-is-operator-configured`), so writing one to make
a page print would send every outbound byte through a host that is not there.

The goal's end gates are clean: `owners.py --closes config-directives-1-3` and
`playbook.py --closes config-directives-1-3` name nothing, and `verify.py --doc` is green.
Nothing is blocked.

## Next group

**Goal `config-directives-2-3`, its first three items — one file set:**
`crates/nvs-config/tests/directives.rs` (`keys_in` lists a block's accepted keys, `governing`
resolves a row, and `DIRECTIVES.iter()` is how a case reads a block's exceptions out of the registry
rather than listing them), `docs/examples/config/<dir>/` and `tests/hostile/config/<dir>/`. The
directory name keeps a key's underscores and turns only its dots into dashes, so ask
`python tools/dossier.py --id '<feature>'` rather than deriving it. All three are secrets or trust
anchors, so one reading of `rule:security/csrf-is-on-by-default` serves the last two.

- [ ] **`directive:http.client.tls`** — `System` and the only `Boot` row under `[http.client]`:
      the one `ClientConfig` every session shares is built once and handed out by `Arc`, so a new
      snapshot has nothing to apply a changed anchor set to. `crates/nvs-config/src/directive.rs:140`,
      `crates/nvs-config/src/tree.rs:610`.
- [ ] **`directive:http.csrf_key`** — `System` and `Reload`, and the ground is the door rather than
      the block: every request's token is verified against this key, so a request that could set it
      would be choosing which forgeries its co-residents accept.
      `crates/nvs-config/src/directive.rs:127`, `rule:security/csrf-is-on-by-default`.
- [ ] **`directive:http.csrf_key_file`** — the same key read out of a file, which is
      `rule:config/a-secret-is-a-file-whose-content-is-the-value`'s spelling and no less the
      operator's for it. `crates/nvs-config/src/directive.rs:128`.

## Backlog

- The remaining two thirds of the directive roster are goals `config-directives-2-3` and
  `config-directives-3-3` — `docs/agent/goals/dossier/`.
- `about.md` is still not counted by the dossier check; goal `the-description-is-owed` switches it
  on — `docs/agent/loop-goal.md` § *The target*.
