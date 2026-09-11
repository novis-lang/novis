# Handoff

## State

**Goal `config-is-written` — stage 0 is green and stage 1 has landed.** `python
tools/directives.py --check` passes at **212 leaf keys** and `--explain storage.<name>.root` exits 0,
so both of stage 0's checks pass. `verify.py` is 10 of 10 green.

**Stage 2's gate exists and the file it gates does not.** `--check-template` catches four things: a
roster key the file omits, a key `tree.rs` does not parse, a key spelled twice, and an `[unread:]`
key with no `# NOT IMPLEMENTED` line naming its owner. A setting is `#key = value` with **no space
after the `#`** — that is what separates it from the prose above it, and a live key is a problem.
It takes an optional path, so a draft is gated where it is written. Today it exits 1 with
`crates/nvs-config/src/default.toml is not written yet`, which is an open item, not a regression.

**The default file cannot be rendered from what the tree already says.** `--json` now carries each
key's `tree.rs` doc comment and its block's, so the generator reads one call instead of `tree.rs`'s
1075 lines — but those comments are maintainer prose: they carry the changeability class and a rule
id and **never the default value**. `crates/nvs-config/src/directive.rs:65`'s row is `key`, `class`,
`apply` and holds no default either, and `docs/reference/tools/20-config.md` covers `[limits]`,
`[mode]`, `[capabilities]`, `[[app]]` and `[[include]]` in operator prose and no other block. So
stage 2 is prose written by hand and held honest by the gate, not a render.

**One decision sits inside stage 2 and the goal already frames it** (`loop-goal.md` § *Stage 2*
point 2): a `Default:` line is true only once an unset key falls back to that value, and an unset
`limits.cpu_time` is no ceiling at all today. Land the fallback first or print what is true — the
safe option is to print what is true, because a comment claiming a ceiling that does not exist is
`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` read backwards.

**Five keys stay declared unread**, each with the rule that closes it — `debug.mode`,
`metrics.listen`, `metrics.endpoint`, `trace.endpoint`, `server.socket_mode`. Each needs its
`# NOT IMPLEMENTED` note in the default file, and `--check-template` fails on any that lacks one.

## Next group

**Stage 2: the default file, against the gate that already holds the roster — one file set:**
`crates/nvs-config/src/default.toml` (new), `crates/nvs-config/src/lib.rs`, `tools/verify.py`,
`crates/nvs-config/tests/directives.rs`.

- [ ] **Write `crates/nvs-config/src/default.toml`, every key commented out.** The shape is exactly
      what `tools/directives.py:624` enforces, and `python tools/directives.py --json` is the whole
      input: 212 rows, each with its block, its `tree.rs` prose and its `[unread:]` owner. A map
      block appears once as a named example — `[db.main]`, `[mail.default]`, `[storage.local]`.
      Run `--check-template` against a draft under `.agent-tmp/` and let its missing-key list drive
      the writing; the file may be inert because
      `rule:config/no-configuration-file-is-a-complete-configuration` makes the shipped defaults a
      complete configuration already.
- [ ] **`include_str!` it as `nvs_config::default_file()`** beside the rest of the crate's surface at
      `crates/nvs-config/src/lib.rs:96`, and add `directives.py --check-template` as a step beside
      `directives` at `tools/verify.py:420` — in the same commit as the file, never before it, or
      every session's verification goes red on an artefact that does not exist yet.
- [ ] **The three tests `loop-goal.toml` stage 2 names**, beside the stage-1 block that ends at
      `crates/nvs-config/tests/directives.rs:294`: `the_default_file_parses_with_deny_unknown_fields`,
      `the_default_file_resolves_to_the_same_snapshot_as_no_file_at_all`, and
      `every_unimplemented_key_in_the_default_file_is_marked_as_one`. The second is the one with a
      trap in it: a live `[limits]` header over no live key still deserializes to `Some(Limits)`
      where no file at all gives `None`, so comment the headers too if the snapshots differ.

## Backlog

- The `limits.cpu_time` fallback — an unset key is no ceiling today (`loop-goal.md` § *Stage 2*).
- Stage 3, a project command writes the file, and its four refusals (`loop-goal.md` § *Stage 3*).
- Stage 4, the record and the rule fragment for what stages 2 and 3 decided (`loop-goal.md` § *Stage 4*).
- `debug.mode` is the one unread key whose subsystem exists — wiring it is a real slice, not a marking.
- `docs/decisions/0175.md` § 4 and `loop-goal.md` § *Standing decisions* both name `E0604` for a
  retired `[cache] dir`; the code the tree emits is `E0601`.
