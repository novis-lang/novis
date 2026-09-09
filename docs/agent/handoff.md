# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls. Stages 0, 2 and 3 are landed, and
stage 4 (the two diagnostics) is next.** Stage 1 is goal 39's floor and carries. Nothing is blocked,
and the goal's § *Standing decisions* still pre-authorizes every call the rest of it reaches.

On disk: `nvs agent primer|index|find|show`, and `python tools/reference.py --primer --check`, which
renders the primer from the built binary into `.agent-tmp/primer/primer.md`, runs the examples it
carries through the same `examples_in`/`run_example` harness the chapters use, and fails a refusal
row whose `E0xxx` is declared nowhere in `crates/nvs-diagnostics/src/lib.rs`. A primer carrying no
example, or naming no code, fails rather than passing vacuously. It currently proves 2 examples and
60 codes.

`rule:tooling/a-primer-claim-is-executed` is now `shipped`, and its fragment states what the tool
does: a refusal is checked as its declared code, because its PHP cell is a fragment no `nvs check`
can be handed. Nothing else in the goal is `designed`.

Measured, and not a gate: the primer is 280 lines and 20 KB — call it 5k tokens, above the low four
figures `docs/decisions/0167.md` § 2 names. The only lever is which sections carry a `<!-- primer -->`
marker; trimming a lifted section's prose is refused by § *Standing decisions*.

## Next group

**Stage 4: the two diagnostics** — one file set: `crates/nvs-hir/src/members.rs` and
`crates/nvs-runtime/src/capability.rs`, each with its own `.nvst` case, because stage 4's `nvs-suite`
check runs `tests/conformance/` whole.

- [ ] **An unknown member suggests the nearest registered name**, where `E_UNDEFINED_MEMBER` is
      raised at `crates/nvs-hir/src/members.rs:1267`. Edit distance over that class's own members,
      one suggestion, and none past a threshold — a confident wrong suggestion is worse than none.
      The three test names the check asks for go in the `#[cfg(test)]` module already asserting on
      that code at `crates/nvs-hir/src/members.rs:1383`: `nvs-hir` has no `tests/` directory, so
      `cargo test -p nvs-hir` reaches them only as unit tests. `docs/agent/loop-goal.md` § *Stage 4*
      item 1 is the specification; no rule owns did-you-mean yet.
- [ ] **A capability denial names where the grant is written**, at
      `crates/nvs-runtime/src/capability.rs:109`, where all four scopes format their message. A
      `help:` line naming `nvs.toml` and the `[capabilities.<name>]` table, added *under* a message
      whose subject and wording do not move — `the_denials_own_subject_and_wording_are_unchanged` is
      the guard that says so. `rule:security/capability-check-at-the-door` owns the refusal itself.

## Backlog

- `python tools/verify.py` does not run `--primer --check`; only the loop driver's acceptance check
  does — `docs/agent/commands.md` § *The user-facing reference, and its proof*.
- Stage 5 — `nvs agent init` and the adapters — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 6 — the rulebook — `docs/agent/loop-goal.md` § *Stage 6*.
- The primer is ~5k tokens against `docs/decisions/0167.md` § 2's budget; the lever is the marker set.
