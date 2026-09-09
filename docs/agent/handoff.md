# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls.** Stages 0, 2, 3 and 4 are on
disk, stage 1 is goal 39's floor, and stage 5 is now two thirds landed: both of its `cargo-named`
checks are green. `nvs agent init` writes the `AGENTS.md` stanza and, beside it, one adapter for each
harness the tree shows — `.claude/skills/novis/SKILL.md` when `.claude/` is present, and every
adapter under `--all`. What is left of stage 5 is the chapter and the cold-start walk. Nothing is
blocked, and the goal's § *Standing decisions* still pre-authorizes every call the rest of it reaches.

**How the adapter rule is kept mechanical rather than remembered:** the protocol prose is one const,
`PROTOCOL` in `crates/nvs-cli/src/agent.rs`, and an `Adapter` row contributes only the header its own
harness reads the file through. There is nowhere for a language fact to be written per harness, which
is `rule:tooling/an-adapter-carries-protocol-and-never-language` as a shape instead of a rule to obey.

**`init` never overwrites.** A stanza or an adapter that still reads as this binary would write it is
left alone; anything else is a refusal naming the file and the way out. An upgrade and a reader's own
edit are indistinguishable from the file, so both refuse — the module doc at
`crates/nvs-cli/src/agent.rs`'s § *The install* is that decision's home.

**This repository has not installed the stanza into its own `AGENTS.md`.** That is stage 5 item 4 and
it is deliberately left for a session that takes it on purpose: it changes what every session reads.

## Next group

**Stage 5: the chapter, and the cold-start walk** — one file set: a new
`docs/reference/tools/50-agents.md`, `tools/reference.py`'s leg table, and `CHAPTERS` in
`crates/nvs-cli/src/agent.rs`.

- [ ] **The chapter `docs/reference/tools/50-agents.md`**, at `crates/nvs-cli/src/agent.rs:98`,
      which is the `CHAPTERS` table the new file joins so the primer's chapter map carries it.
      Front matter and runnable examples in the shape its four siblings under
      `docs/reference/tools/` have; `docs/agent/loop-goal.md` § *Stage 5* item 3 is the
      specification and `rule:tooling/an-agent-asks-the-binary` is what it documents.
- [ ] **`python tools/reference.py --agent-walk`**, at `tools/reference.py:791`, where `--primer`
      declares the leg beside it. The walk is primer, `find`, `show`, `nvs check`, and it fails when
      any step returns nothing that resolves. The leg does not exist yet;
      `docs/agent/loop-goal.toml:6652` is the check that names it.
- [ ] **This repository installs it too**, at `crates/nvs-cli/src/agent.rs:674`, which is the `init`
      that writes it — run the built `nvs agent init` at the repository root so the root `AGENTS.md`
      gains the stanza and the Claude skill lands beside it.
      `docs/agent/loop-goal.md` § *Stage 5* item 4. Take it deliberately: it widens the file that is
      inlined into every session's context.

## Backlog

- `rule:tooling/an-adapter-carries-protocol-and-never-language` is still `status: designed` with no
  `guardedBy`; `crates/nvs-cli/tests/agent.rs` now holds it — `docs/rules/tooling.json`.
- `rule:tooling/an-agent-asks-the-binary` flips to `shipped` when stage 5's chapter lands —
  `docs/rules/tooling.json`.
- Adapters for harnesses other than Claude Code: one row in `ADAPTERS`, decides nothing —
  `crates/nvs-cli/src/agent.rs`.
- Carried gaps that outlive this goal are in `docs/agent/carried-gaps.md`.
