# Handoff

## State

**Goal 40 — An agent learns Novis from the binary, in three calls — has just started; nothing of it
has landed yet.** Goal 39's whole list is this goal's Stage 1 floor.

The design is settled and frozen: [0167](../decisions/0167.md) holds it, and the goal's
§ *Standing decisions* names every tradeoff a session would otherwise stop on. Three things a session
must not re-decide:

- **`nvs agent`, not an extension of `nvs doc`.** A symbol is not a path, and `nvs doc` takes one
  positionally.
- **No per-member capability field.** `rule:security/capability-declaration-is-one-table` refuses
  that shape for a security-review reason, and stage 2 joins against the table at render time
  instead. The table itself is read and never edited.
- **Nothing cached, nothing written to disk** by the four read verbs. An answer that can be stale is
  the failure the surface exists to remove.

Stage 0 is an audit and is cheap: two `tooling` rules are marked `designed` over surface the binary
appears to ship (`nvs doc <entry> --out <dir>` writes pages today; `nvs meta --json <entry>` emits a
`program` key today), and one `shipped` rule — `security/capability-declaration-is-one-table` — names
two renderers that do not exist. Confirm each against the binary before stage 2 builds on it.

## Next group

**Stage 0 then stage 2** — one file set: `crates/nvs-cli/src/{main,meta}.rs`, reading
`crates/nvs-stdlib/src/registry.rs`. Stage 0 is an audit with no edit outside the rulebook, so it is
cheap to take in front of the keystone rather than in a session of its own.

- [ ] **Audit the three rules named above** against the release binary — `nvs doc`, `nvs meta --json
      <entry>`, and whether any renderer reads `CAPABILITIES`. Flip the `designed` ones that hold;
      write down, and leave `designed`, any that does not.
- [ ] **`crates/nvs-cli/src/meta.rs`** — the document gains a `capabilities` roster rendered from
      `crates/nvs-stdlib/src/registry.rs`'s `CAPABILITIES`: class, member, capability. A seventh
      top-level key beside `exceptions`, `interfaces`, `attributes` and `directives`. No field is
      added to a member row.
- [ ] **`crates/nvs-cli/src/agent.rs`** (new) with `index`, and `crates/nvs-cli/src/main.rs:186`'s
      `Command` enum gaining `Agent`. One line per member joined to that roster, so
      `Core\IO::read(string $path): string  [fs.read]` is what a line looks like.
- [ ] **`find` and `show`** beside it, if the group still has room — same file, and the tests for all
      three are one `-p nvs-cli` run.

## Backlog

- **Stage 3, the primer** — `crates/nvs-cli/src/agent.rs`, `docs/reference/lang/*.md` markers and
  `tools/reference.py`. Shares one file with the group above and adds two; take it next.
- **Stage 4, the two diagnostics** — `crates/nvs-hir/src/members.rs:1014` and
  `crates/nvs-runtime/src/capability.rs:109`. Shares nothing with the rest of this goal; it is a
  session of its own and that is the right outcome.
- **Stage 5, the install** — `crates/nvs-cli/src/agent.rs` again, plus a new
  `docs/reference/tools/50-agents.md`.
- **Stage 6, the rulebook** — including `rule:tooling/meta-json`'s "Four rosters" sentence, which
  stage 2 makes five and which stays true until then.
- When this goal's last check goes green the driver takes goal 26.
  `docs/agent/goals/chain.toml` is the schedule and this does not restate it.
