# Handoff

## State

Goal `config-directives-1-3` is **met**: `python tools/dossier.py --verify` over its 16 features
reports nothing owed, 0 failed examples and 0 failed hostile cases, and the floor check that held the
previous DONE claim is green. That check was `tools/check-links.py`, and what it found was real — a
usage line in `tools/orient.py` named `docs/agent/goals/71-dossier.toml`, which went with goal
`dossier`'s retirement; the line now spells its argument `<goal>.toml` like every other usage
placeholder in this repository. `owners.py --closes`, `playbook.py --closes` and `verify.py --doc` are
all clean for the goal.

Working ahead, goal `config-directives-2-3`'s first three features are complete — `http.client.tls`,
`http.csrf_key` and `http.csrf_key_file`, each with a census test in
`crates/nvs-config/tests/directives.rs`, one example with a blessed `.out`, one attack and an
`about.md`. `nvs.toml` gained `[http.client.tls] min_version = "1.2"`, which is the floor the client
already speaks, so the page prints a version rather than nothing; `roots` and `keylog` stay unwritten
and the comment above the block is where that is recorded. Neither `[http] csrf_key` page shows a key
in force, on purpose: writing one into this checkout would arm the door's token half for every
fixture in the tree, and an inline key in a shipped configuration is the spelling
`rule:config/a-secret-is-a-file-whose-content-is-the-value` tells a deployment not to use.

Nothing is blocked.

## Next group

**Goal `config-directives-2-3`, its items 4 to 6 — one file set:**
`crates/nvs-config/tests/directives.rs` (`keys_in` lists a block's accepted keys, `governing` resolves
a row, and a sweep over `DIRECTIVES.iter()` is how a case reads a block's exceptions out of the
registry rather than listing them), `docs/examples/config/<dir>/` and `tests/hostile/config/<dir>/`.
The directory name keeps a key's underscores and turns only its dots into dashes, so ask
`python tools/dossier.py --id '<feature>'` rather than deriving it.

- [ ] **`directive:include`** — the one directive that decides which *files* the configuration is,
      so an attack on it is an attack on every value in the tree.
      `crates/nvs-config/src/directive.rs:257`, `crates/nvs-config/src/resolve.rs:1`.
- [ ] **`directive:io.temp_root`** — `Boot`, and the pair to `debug.keep_temporary`, which already has
      its page: the root is fixed where the keeping is the operator's per reload.
      `crates/nvs-config/src/directive.rs:207`.
- [ ] **`directive:limits`** — the blanket row over the block whose `hard` half is item 9, so the two
      are one reading: what a request may raise for itself, and the ceiling it is raised against.
      `crates/nvs-config/src/directive.rs:94`.

## Backlog

- Items 7 to 16 of goal `config-directives-2-3` — the rest of `[limits]` and the whole of `[log]`;
  the list with anchors is `docs/agent/goals/dossier/73-config-directives-2-3.md`.
- A `[http] csrf_key` page that shows a key actually in force would need a fixture tree of its own
  rather than the repository's `nvs.toml`; nothing owes it, and both pages say what an operator sees.
