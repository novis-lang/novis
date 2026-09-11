# Handoff

## State

**Goal `config-is-written` — stage 3 is complete on disk.** Each of its three checks names tests
that now exist and pass: the write, the four refusals, and the declined record.

**A declined write carries its reason as a value.** `crates/nvs-cli/src/config.rs:178`'s `Declined`
is the three answers the write can give — `Untrusted`, `Exists`, `Unwritable` — and its `note`
renders the one line each.

**`nvs init` is the explicit door, and the only place that reason is reported.**
`crates/nvs-cli/src/main.rs:471`'s `Command::Init` reaches `crates/nvs-cli/src/config.rs:261`'s
`init` — the same write with the opposite failure policy, an `error:` and a non-zero exit, because
this command exists only to produce the file. It is deliberately not in the `initializes` table.

**Stage 3 § 7 is what stage 3 still owes, and it is blocked on another rule.**
`crates/nvs-cli/src/config.rs:331`'s `boot_in` stays silent either way: nothing in `nvs-cli` prints
a configuration line at boot, because `rule:config/the-resolved-root-is-announced-and-stored` is
`designed`. A `note:` of its own was written and taken back out — see the playbook bullet under
*Running things*. Where it belongs is that rule's boot line, not a second printing site.

**One open design consequence, carried for stage 4's record.** `nvs check` is in the writing table,
so the *second* `nvs check` in a fresh directory reads a tree that grants nothing and refuses a
literal `Core\Db::open` host the first one passed —
`rule:config/no-configuration-file-is-a-complete-configuration`'s last paragraph.
`crates/nvs-cli/src/config.rs:404`'s `grants` asks about the roots as they were **found**, so a file
this invocation manufactured is still no tree, which is what keeps
`crates/nvs-cli/tests/check_grants.rs:88` true for run one. Run two is covered only by deciding
which landed statement gives way.

**The floor check is red on a file no session wrote.** `python tools/chain.py --check` fails on
`docs/agent/goals/50-outbound-proxy.toml` — empty `tests`, TODO markers, no README row — an
uncommitted by-hand scaffold. Filling it in is its author's call.

## Next group

**Stage 4: the records and the rulebook — one file set:** `docs/decisions/`,
`docs/rules/config.json` with its fragments under `docs/rules/config/`, and `nvs.toml`.

- [ ] **The decision record this goal owes.** Stage 2's commented-versus-live argument, stage 3's
      command split and its four refusals, stage 1's `cache.dir` migration, and the run-two
      consequence above under `## Consequences`. The shape is `docs/agent/conventions.md:1`
      § *A decision record*; re-derive the next free number from `docs/decisions/` before claiming
      it, because this tree has more than one writer. The code it describes is
      `crates/nvs-cli/src/config.rs:178` and `crates/nvs-cli/src/main.rs:471`.
- [ ] **`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` gains step 3's
      write.** `docs/rules/config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults.md:1`,
      with step 2 and *never a walk upward* left exactly as they stand — the goal adds what happens
      *at* step 3 and adds no lookup. Its `because` gains the new record's number, and the record's
      `changes.modifies` names it back.
- [ ] **`rule:config/no-configuration-file-is-a-complete-configuration` gains one sentence.**
      `docs/rules/config/no-configuration-file-is-a-complete-configuration.md:1`: the shipped
      defaults stay a complete configuration, and are what a run uses whenever the file could not be
      written.
- [ ] **Stage 0's gate becomes a fragment, and the repository's own configuration file stops citing
      ADR numbers.** The fragment joins `docs/rules/config.json:1`'s array at the position
      `docs/agent/conventions.md:1` § *Where a rule sits in the order* gives it; the tree's root
      `nvs.toml` still carries `ADR 0103 § 1 step 2` and `ADR 0118 § 2` from before the docs
      migration. Then `python tools/rules.py --render`, which `session.py --wrap` runs for any
      session that touched `docs/rules/` — a fragment written without one refuses the wrap.

## Backlog

- Stage 3 § 7's boot line, blocked on `rule:config/the-resolved-root-is-announced-and-stored`.
- `docs/agent/goals/50-outbound-proxy.*` is an unfilled scaffold; `chain.py --check` is red on it.
- `docs/agent/goals/45-fmt.toml` and `46-template-format.toml` hold prose to a rule no `rules` entry
  reaches (`chain.py --check` notes).
