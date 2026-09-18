# Handoff

## State

Goal `config-directives-2-3`, milestone dossier. **All sixteen items are complete**: the goal's own
check, `dossier: config:directives (2/3)`, reports nothing owed and both suites at 0 failed. Nothing
is blocked.

Three things about the tree changed under these five items, and none of them is a proof:

- The repository-root `nvs.toml` now states `[limits] max_script_depth` and a `[log]` block holding
  the two handler reserves, at exactly the figures the code ships, so those pages print real values
  and nothing in the repository behaves differently. The block deliberately states no `format`,
  `level` or `handler`: the first two are the mode's to decide and the third is written per
  application.
- `crates/nvs-config/src/default.toml` documented the tier-3 reserves at `4M` and `500ms`. The
  shipped figures are 16 MiB and 5 s (`crates/nvs-runtime/src/ctx/isolate.rs:473`), and both the
  `[log]` and `[app.log]` blocks say so now.
- A written `[limits] max_script_depth = 0` read as the no-ceiling sentinel, against its own field
  doc, so an operator writing the number most likely to mean *no nesting* got no bound at all — a
  probe recursed 56,395 isolates deep. It takes the default now
  (`crates/nvs-runtime/src/ctx/limits.rs:586`).

## Next group

**Stage 2: the dossier — one file set: `crates/nvs-config/src/directive.rs` and
`crates/nvs-config/tests/directives.rs`, plus each feature's own three proof paths.** What is left of
`[log]` is the floor's destination, and it belongs to the next goal in the chain rather than to this
one. `python tools/dossier.py --id '<feature>'` prints the three paths; check an item with it before
taking it.

- [ ] **`directive:log.target`** — owes examples, hostile, tests. Tier 4, the floor that is always
      there: `stderr`, `file:<path>` or `syslog`, and `System` because `rule:errors/engine-floor`
      says the sink is operator-owned in as many words.
      `crates/nvs-config/src/directive.rs:178`
- [ ] **The rest of `config:directives (3/3)`** — whatever that goal's `--only` list names beyond
      the `[log]` block; the registry rows it walks start at
      `crates/nvs-config/src/directive.rs:185`

## Backlog

- A request-set `limits.cpu_time` is accepted and never charged under `nvs run`
  (`crates/nvs-cli/src/main.rs:2459`); a file-written one is. Nothing owns this.
- `Core\Config::set` accepts any word for an enumerated key — `set('log.format', 'banana')` answers
  `true` (`crates/nvs-config/src/request.rs:101` checks a unit, a ceiling and the `[http]` pairs).
- `directive:log.target` and the rest of `config:directives (3/3)`, above.
