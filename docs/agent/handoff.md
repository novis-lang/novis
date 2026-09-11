# Handoff

## State

**Goal `config-is-written` — stage 0 has landed and nothing else of the goal has.**
`tools/directives.py` walks `Config`'s field graph in `crates/nvs-config/src/tree.rs` and derives
**213 leaf keys**, giving a map block the `<name>` segment `[db.<name>]`, `[mail.<name>]` and
`[storage.<name>]` are written with. It is step 3 of `python tools/verify.py`, beside `lints`.

**The gate is red, on purpose, and stopping the rest of the gate with it.** Six keys reach no
reader and declare nothing: `debug.mode`, `metrics.listen`, `metrics.endpoint`, `trace.endpoint`,
`server.socket_mode`, `cache.dir`. Deciding them is the next group; the playbook bullet says what
to run in the meantime. This session touched no Rust — `verify.py --fast` is green for build and
test, and `test`, the `.nvst` trees, `reference` and `clippy` did not run.

**The tool's six is not the hand-audit's nine, and the difference is the question being asked.**
`capabilities.debug.trace`, `capabilities.debug.profile`, `extension.path` and `extension.sha256`
each reach a reader — `Cap::grant`'s field map and the artifact cache's `env_hash` — so this gate
counts them read. *Nothing acts on the value* is a second question, answered by hand and not by
this walk; the tool's own doc comment says so, and stage 1 decides whether it wants those four
tracked anywhere.

**The three ways a reader is counted are all load-bearing** and each is the only reader of at
least one shipped key — the tool's doc comment holds that argument, along with why a row in
`directive.rs` or `capability.rs` names a key without reading it.

## Next group

**Stage 1: decide each key the gate names — one file set:** `crates/nvs-config/src/tree.rs`,
`crates/nvs-config/src/directive.rs` and `crates/nvs-cli/src/cache.rs`. Every item ends the same
way: a reader, a deletion, or an `[unread: <why> owner: <who>]` trailer on the field's own doc
comment, after which `python tools/directives.py --check` is green and the gate reaches `build`
again.

- [ ] **`cache.dir` — decide which spelling of the artifact cache's directory survives**, and
      write the decision record for it: `rule:config/three-changeability-classes` classifies it
      `System`/`Boot` at `crates/nvs-config/src/directive.rs:150`, while the cache itself reads
      `opcache.file_cache_dir` at `crates/nvs-cli/src/cache.rs:@from_config`. The field is
      `crates/nvs-config/src/tree.rs:962`. Removing a key from a `deny_unknown_fields` struct turns
      a file that parses today into an `E0604` tomorrow, so this one owes a migration note.
- [ ] **`metrics.listen`, `metrics.endpoint` and `trace.endpoint` name addresses nothing binds or
      pushes to** — `rule:observability/a-registry-is-per-core-and-nothing-reads-it` is why, and
      `crates/nvs-server/src/metrics.rs`'s module doc says it in four words. The fields are
      `crates/nvs-config/src/tree.rs:875`, `:877` and `:889`; the trailer's owner is the milestone
      that lands an exporter.
- [ ] **`server.socket_mode` reaches no code in the workspace at all** — decide it against
      `rule:config/ownership-is-the-trust-boundary`, which is what a socket's mode is for, at
      `crates/nvs-config/src/tree.rs:906`.
- [ ] **`debug.mode` is the key the hand-audit missed**, `RuntimeTighten` per
      `rule:testing/debug-mode-directive` and read by nothing, at
      `crates/nvs-config/src/tree.rs:379`.

## Backlog

- Stage 2 — generate the commented-out `nvs.toml` from `--json`; goal `config-is-written`'s § *Standing decisions* fixes its shape.
- Stage 2 — the five project commands that write it, as a table rather than a flag; same § of the goal.
- `[context] shapes` in `docs/agent/loop-goal.toml` is missing *A playbook bullet*, so the trailer grammar cost a read of `tools/playbook.py`.
- `[context] modules` names `tools/lints.py` and `tools/verify.py` but not `tools/splice.py`, whose patch format cost a `--help`.
- Four keys the gate calls read but nothing acts on — `capabilities.debug.{trace,profile}`, `extension.{path,sha256}`; `docs/agent/carried-gaps.md` if stage 1 does not take them.
