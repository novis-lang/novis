# Handoff

## State

**Goal `config-is-written` — stages 0 and 1 have landed.** `python tools/directives.py --check` is
green at **212 leaf keys**, so the gate's third step passes and `verify.py` reaches `build` again.
Stage 2 — the generated, commented-out default file — is untouched.

**The artifact cache's directory has one spelling.** `opcache.file_cache_dir` survives and carries
the `System`/`Boot` row `cache.dir` held, as a row longer than `[opcache]`'s so `lookup` finds it
first; `[cache]` is `Core\Cache`'s `local` and `shared` tiers and holds no third key. Writing
`[cache] dir` is `E0601` — **not `E0604`**, which is the duplicate-key code; the goal prose and the
goal's § *Standing decisions* both say `E0604` and both are wrong about the code, not about the
migration. `docs/decisions/0175.md` is the record, § 4 the migration note.

**Five keys are declared unread rather than wired**, each with the rule that closes it as its
`owner:` — `debug.mode`, `metrics.listen`, `metrics.endpoint`, `trace.endpoint`,
`server.socket_mode`. `debug.mode` is the one whose subsystem already exists: the probes are
compiled in and `DebugFlags` arms them, but no directive row governs the key and nothing reads it,
so wiring it is a real slice and not the marking stage's work.

## Next group

**Stage 2: the default file, gated by the tool that already holds the roster — one file set:**
`tools/directives.py`, `crates/nvs-config/src/default.toml` (new), `crates/nvs-config/src/lib.rs`,
`crates/nvs-config/tests/`.

- [ ] **`tools/directives.py --check-template` — write the gate before the file it gates**, the way
      stage 0 wrote the roster before deciding anything: every leaf key appears exactly once, no key
      the tree does not parse appears at all, and every `[unread:]` key sits under a
      `# NOT IMPLEMENTED` line naming its owner. It reuses the walk at `tools/directives.py:449` and
      the trailer pattern at `tools/directives.py:102`, and it is `loop-goal.toml`'s first stage-2
      check. `rule:config/no-configuration-file-is-a-complete-configuration` is why the file may be
      inert: the shipped defaults are already a complete configuration.
- [ ] **Generate `crates/nvs-config/src/default.toml`, every key commented out**, `include_str!`'d
      and reached as `nvs_config::default_file()` from `crates/nvs-config/src/lib.rs`. Shape and the
      `cpu_time`/`wall_time` comments are goal prose § *Stage 2* 1–4; the roster and each key's
      trailer come from `tools/directives.py:510`'s `--json`, and
      `crates/nvs-config/src/tree.rs:376` is a marked key to render against.
- [ ] **The three tests `loop-goal.toml` names**, beside the block sweep at
      `crates/nvs-config/tests/tree.rs:69`: the file parses under `deny_unknown_fields`, it resolves
      to the same snapshot as no file at all (`rule:config/no-configuration-file-is-a-complete-configuration`),
      and every unimplemented key in it is marked as one.

## Backlog

- Stage 3 — the five project commands that write the file, as a table not a flag; goal § *Standing decisions*.
- `rule:testing/debug-mode-directive` is `shipped` while `[debug] mode` reaches no reader and no directive row; the status or the wiring is wrong, and either needs its own record.
- Stage 2 § 2 — an unset `cpu_time` is no ceiling today, so the file's `Default:` lines are true only once that fallback lands.
- The four keys the gate counts read but nothing acts on — `capabilities.debug.{trace,profile}`, `extension.{path,sha256}`; `docs/agent/carried-gaps.md` if stage 2 does not take them.
- `[context] modules` in `docs/agent/loop-goal.toml` omits `tools/directives.py` and `tools/splice.py` — this session paid a read for the trailer grammar and a `--help` for the patch format.
- `[context] rules` omits `config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`, which is the one home of `E0601` versus `E0604` and the fact the goal prose gets wrong.
