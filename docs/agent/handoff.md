# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls.** Stage 1 is goal 39's floor;
stages 0, 2, 3 and 4 are on disk, and stage 5's three acceptance checks now all pass. `nvs agent`
is the four read commands plus `init`, the surface has a chapter of its own at
`docs/reference/tools/50-agents.md`, and `python tools/reference.py --agent-walk` walks it cold —
primer, `find`, `show`, `nvs check` — handing each step only what the step before it printed.

**The primer opens on the new chapter.** The lookup-protocol section moved out of
`docs/reference/tools/10-cli.md`, which keeps only its subcommand row, and `PRIMER_FIRST` in
`crates/nvs-cli/src/agent.rs` names `agents` where it named `cli`. A `<!-- primer -->` marker
decides *that* a section is lifted; that list decides *where* it lands, so a section moved between
chapters without it reorders the primer silently.

**One item of the goal is left and no check names it**: `docs/agent/loop-goal.md` § *Stage 5*
item 4, this repository installing the stanza into its own `AGENTS.md`. It stays deliberate because
it changes what every session reads, and because `init` in this tree also writes
`.claude/skills/novis/SKILL.md`, which is a file in the user's own harness directory. Nothing is
blocked.

## Next group

**Stage 5: the install, in this repository** — one file set: `AGENTS.md`,
`crates/nvs-cli/src/agent.rs`, `crates/nvs-cli/tests/agent.rs`.

- [ ] **This repository installs the stanza**, at `crates/nvs-cli/src/agent.rs:675`, which is
      `init` — run the built binary at the repo root rather than writing `AGENTS.md` by hand, so
      what lands is byte-for-byte what `init` writes and re-running stays a no-op.
      `docs/agent/loop-goal.md` § *Stage 5* item 4 is the specification and
      `rule:tooling/an-adapter-carries-protocol-and-never-language` is what the stanza carries.
      The same run writes `.claude/skills/novis/SKILL.md` because `.claude/` is present: take that
      on purpose and say which way in the commit.
- [ ] **A test that this tree's own stanza has not drifted**, at
      `crates/nvs-cli/tests/agent.rs:414`, beside the `init` cases: this repository's `AGENTS.md`
      still holds the block this binary writes, so an edit to `PROTOCOL` that forgets the installed
      copy fails here rather than in a reader's context. `init` refuses to overwrite one it did not
      write, so the way out is delete-and-re-run — name it in the failure message.

## Backlog

- `nvs agent init --embed` is not built and is not to be built — `docs/decisions/0167.md`
  § *Revisiting*.
- The goal's `[context]` manifest can print a rule, a record section, a module doc, a playbook
  bullet or a `conventions.md` shape, and no other `docs/` section: writing a reference chapter
  needed `docs/reference/README.md` § *Examples: the fence grammar* and one sibling chapter, both
  read by hand — `docs/agent/loop-goal.toml:88`.
- `AGENTS.md` § *Where to look* routes three questions, and "what does this `Core` member do" is
  not one of them; the stanza's arrival is when to decide whether it should be — `AGENTS.md`.
