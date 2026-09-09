# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls. Stage 4 is landed whole**, so
stages 0, 2, 3 and 4 are on disk and stage 1 is goal 39's floor. Stage 5 — `nvs agent init`, the
adapters and the chapter — is the only one left. Nothing is blocked, and the goal's § *Standing
decisions* still pre-authorizes every call the rest of it reaches.

A capability denial is now two lines: its own sentence, unchanged, over `help: grant it in nvs.toml
under `[capabilities.<family>]``. The table is `Cap::family` at
`crates/nvs-config/src/capability.rs:275`, which splits the dotted name rather than listing the
blocks a second time. **Why it rides on `$e->message` rather than on the record an uncaught throw
renders** is written at `crates/nvs-runtime/src/capability.rs:@denial`: a denial is catchable, so
most are read by a program that logged the message, and a help line the floor owned would be missing
from exactly the path an operator debugs from. The price is that a program comparing `$e->message`
against a literal compares two lines — which is what the corpus pass cost, and the playbook bullet
now names the reference chapters it also costs.

## Next group

**Stage 5: `nvs agent init` and the adapters** — one file set: `crates/nvs-cli/src/agent.rs`,
`crates/nvs-cli/src/main.rs`'s `AgentCommand` dispatch, and a new `docs/reference/tools/50-agents.md`.

- [ ] **`nvs agent init` writes the `AGENTS.md` stanza**, at `crates/nvs-cli/src/main.rs:1014`,
      where the other four arms already dispatch into `agent.rs`. Re-running is idempotent and it
      refuses to clobber a stanza a user has edited; `docs/agent/loop-goal.md` § *Stage 5* item 1 is
      the specification, and `rule:tooling/an-agent-asks-the-binary` is the surface it installs.
- [ ] **One adapter per harness found**, beside the stanza — a Claude Code skill at
      `.claude/skills/novis/SKILL.md` when `.claude/` is present, `--all` for every adapter
      regardless — written next to `crates/nvs-cli/src/agent.rs:213`, which is where the surface's
      own renderers live. **No adapter states a language fact**:
      `rule:tooling/an-adapter-carries-protocol-and-never-language`, and § *Standing decisions*
      refuses the argument for one.
- [ ] **The chapter `docs/reference/tools/50-agents.md`**, in the shape its sibling
      `docs/reference/tools/20-config.md:1` has, plus this repository's own stanza in `AGENTS.md`
      (§ *Stage 5* items 3 and 4). Its fenced `nvs` blocks run under `verify.py`'s `reference` leg
      like every other chapter's, and a chapter edit owes `python tools/reference.py --no-examples`
      in the same commit.

## Backlog

- The pack's `[context] playbook` filters traps to the paths the *item* names, so the denial-sentence
  bullet — the one that priced this whole slice — was filtered out for naming `tests/` paths while
  the item named `crates/`. Naming that bullet directly in `[context] playbook` is the fix.
- The primer is ~5k tokens against the low four figures `docs/decisions/0167.md` § 2 names, and the
  only lever is which chapter sections carry a `<!-- primer -->` marker.
- No rule owns did-you-mean or the denial's help line; both are documented at their code
  (`crates/nvs-hir/src/members.rs:1354`, `crates/nvs-runtime/src/capability.rs:@denial`) and pinned
  by conformance cases, since this goal opens no ADR number.
