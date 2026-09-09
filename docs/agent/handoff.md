# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls. Stages 0 and 2 are landed, and
stage 3 all but its executed check.** Stage 1 is goal 39's floor and carries. Nothing is blocked,
and the goal's § *Standing decisions* still pre-authorizes every call the rest of it reaches.

On disk: `nvs agent primer|index|find|show`. `primer` prints the chapter sections a
`<!-- primer -->` marks and no sentence of its own about the language — the lookup protocol from
`docs/reference/tools/10-cli.md`, one worked program from `docs/reference/lang/10-programs.md`,
`[capabilities]` from `docs/reference/tools/20-config.md`, seven refusal tables from
`docs/reference/tools/30-php-differences.md` — then the chapter map from all fifteen chapters' own
front matter. The chapters are embedded whole at `crates/nvs-cli/src/agent.rs:78`;
`docs/reference/README.md` § *Marking a section for the primer* is the marker's home.

`rule:tooling/an-agent-asks-the-binary` is now `shipped`, all four verbs being in the binary and
guarded. `rule:tooling/a-primer-claim-is-executed` stays `designed` until the check below lands.

Measured, and not a gate: the primer is 280 lines and 20 KB — call it 5k tokens, above the low four
figures `docs/decisions/0167.md` § 2 names. The only lever is which sections carry a marker;
trimming a lifted section's prose is refused by § *Standing decisions*.

## Next group

**Stage 3: every primer claim executed** — one file set: `tools/reference.py` and
`crates/nvs-cli/tests/agent.rs`.

- [ ] **`python tools/reference.py --primer --check` runs the primer's own examples**, the flag
      beside the others at `tools/reference.py:715`. Write `nvs agent primer` to a file under
      `.agent-tmp/`, then reuse `tools/reference.py:604`'s `examples_in` and
      `tools/reference.py:642`'s `run_example` unchanged — `rule:tooling/a-primer-claim-is-executed`
      asks for the harness `docs/novis.md`'s examples already use, not a second one, and the worked
      program is the example it finds.
- [ ] **A refusal claim is checked as its code, not as a program.** A lifted table's PHP cell is a
      fragment that no `nvs check` can be handed, so what is executable about it is the `E0xxx` in
      its third column: every code a lifted section names must be declared beside its siblings at
      `crates/nvs-diagnostics/src/lib.rs:67`. Same command, same `--check`.
- [ ] **`tools/reference.py`'s module doc gains the primer**, at `tools/reference.py:2`, which today
      says the tool builds `docs/novis.md` and stops.

## Backlog
- Stage 4, the two diagnostics: `crates/nvs-hir/src/members.rs:1014` and
  `crates/nvs-runtime/src/capability.rs:109` (`docs/agent/loop-goal.md` § *Stage 4*).
- Stage 5's adapters, `nvs agent init` — `rule:tooling/an-adapter-carries-protocol-and-never-language`.
- `tools/reference.py` prints no capability beside a member's card, which
  `rule:security/capability-declaration-is-one-table` names as a consumer of the table.
- `docs/reference/tools/10-cli.md` still has no section for `nvs serve`, `queue`, `schema`, `lsp`
  or `doc`, which its own "not in this chapter yet" paragraph admits.
