# Handoff

## State

Goal `config-directives-2-3`, milestone dossier. Four of its sixteen items are complete and the rest
are untouched. Nothing is blocked.

Its items 1–3 — `directive:http.client.tls`, `directive:http.csrf_key`, `directive:http.csrf_key_file`
— had already been landed by the sessions of goal `config-directives-1-3` before this goal was
installed, so the handoff that named them as the next group was naming finished work.
`python tools/dossier.py --id '<feature>'` reports all three `complete.`; this session verified that
and took items 4, 5, 6 and 9 instead. **Check an item with `--id` before taking it**: this goal's
list was emitted before the previous goal's last sessions committed, so an item near the top of it
may already be done.

The four that landed are `directive:include`, `directive:io.temp_root`, `directive:limits` and
`directive:limits.hard`, each with its `about.md`, one example with a blessed `.out`, one attack, and
one Rust case in `crates/nvs-config/tests/directives.rs`. No proof found a bug: the ceiling holds
against every unit spelling and every alternative name for `[limits.hard]` that the attack reaches
for, and `'0'` is the only surprising acceptance — it lowers a budget, so it is a request tightening
itself.

## Next group

**One file set: `crates/nvs-config/src/directive.rs`, `crates/nvs-config/src/tree.rs` and
`crates/nvs-config/tests/directives.rs`, plus each feature's own three proof paths.** These are the
four keys carved out of `[limits]` that are the operator's, in two pairs sharing one reason each, so
the second of a pair costs a fraction of the first. `rule:testing/four-proofs` is what each owes;
`python tools/dossier.py --id '<feature>'` prints the paths.

- [ ] **`directive:limits.fatal_reserve_memory`** — owes examples, hostile, tests.
      `rule:errors/on-limit`'s reserved slice: the bytes left for the tier-1 handler when the rest of
      the ceiling is gone. `crates/nvs-config/src/directive.rs:100`
- [ ] **`directive:limits.fatal_reserve_time`** — the other half of that slice, same rule and same
      class, so it is written against the case above. `crates/nvs-config/src/directive.rs:101`
- [ ] **`directive:limits.max_decompressed`** — `rule:core-classes/decompression-bound`: the most any
      one decompression may produce, and `false` does not remove it.
      `crates/nvs-config/src/directive.rs:111`
- [ ] **`directive:limits.max_decompression_ratio`** — the ratio half of that same bound.
      `crates/nvs-config/src/directive.rs:112`

## Backlog

- `directive:limits.max_script_depth` — the last `System` key inside `[limits]`, and the one with a
  shipped default. `crates/nvs-config/src/directive.rs:105`
- `directive:log`, `directive:log.handler`, `directive:log.handler_reserve_memory`,
  `directive:log.handler_reserve_time` — items 13–16 of this goal, a file set of their own.
- No `benches/members/config/*.nvs` exists for any directive; `tools/data/dossier-policy.toml` does
  not ask a directive for one, so this is the policy rather than a gap.
