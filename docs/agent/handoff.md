# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls. Stages 0, 2 and 3 are landed, and
stage 4 is half landed:** the did-you-mean diagnostic is on disk, the capability help line is not.
Stage 1 is goal 39's floor and carries. Nothing is blocked, and the goal's § *Standing decisions*
still pre-authorizes every call the rest of it reaches.

On disk for stage 4: `E_UNDEFINED_MEMBER` (`E0309`) now carries a `help: did you mean` line when a
member of the same kind, on that class or on an ancestor it reaches through `extends`/`implements`,
is within the edit budget at `crates/nvs-hir/src/members.rs:1385` — one edit for a short name, two
once a transposition costs that much, and never three, because `disconnect` is three from `connect`
and an agent handed a confident wrong name will write it. A member of an unrelated class is never a
candidate. `tests/conformance/class/an-unknown-member-suggests-the-nearest-name.nvst` pins the near
miss, the far one and the unrelated class in one exact rendering.

The other half of stage 4 is a bigger slice than its one sentence reads, and the group below is
sized for that: the goal prose puts the help line *under* the denial, which makes the thrown
message two lines, and that sentence is pinned by 27 conformance cases — the playbook bullet has
the measurement. Deciding whether the line rides on `$e->message` or on the record an uncaught
throw renders is the first thing that slice does; `Fault::Thrown` at `crates/nvs-runtime/src/abi.rs:103`
carries a message and nothing else today, so the second option is a wider change than the corpus pass.

## Next group

**Stage 4: the capability denial's help line** — one file set: `crates/nvs-runtime/src/capability.rs`
and the `tests/conformance/{cap,core}` cases that pin its sentence, because stage 4's `nvs-suite`
check runs `tests/conformance/` whole.

- [ ] **The denial names where the grant is written**, at `crates/nvs-runtime/src/capability.rs:106`,
      which is the one author of the sentence for every door
      (`rule:security/capability-check-at-the-door` § 5, and
      `rule:security/denial-is-a-runtime-error` for why it is catchable). A `help:` line naming
      `nvs.toml` and the capability's own table — `[capabilities.fs]` for `fs.read`, the family half
      of `Cap::name` at `crates/nvs-config/src/capability.rs:256`. `docs/agent/loop-goal.md`
      § *Stage 4* item 2 is the specification; the subject and wording of the first line do not move.
- [ ] **The two `-p nvs-runtime` tests the check names** —
      `an_ungranted_call_names_the_config_file_and_the_capability_table` and
      `the_denials_own_subject_and_wording_are_unchanged` — in the `#[cfg(test)]` module at
      `crates/nvs-runtime/src/capability.rs:1039`, which already builds a `Ctx` from a written
      `nvs.toml` through `snapshot_of` at `crates/nvs-runtime/src/capability.rs:1054`.
- [ ] **The corpus pass**, starting at
      `tests/conformance/cap/an-ungranted-capability-throws-naming-it.nvst:23` and covering every
      file `grep -rl "which is not granted" tests/` names, plus the capability-help case the stage's
      `nvs-suite` check asks for by name.

## Backlog

- Stage 5 — the install and the adapters — is the next stage; `docs/agent/loop-goal.md` § *Stage 5*.
- The primer is ~5k tokens against the low four figures `docs/decisions/0167.md` § 2 names, and the
  only lever is which chapter sections carry a `<!-- primer -->` marker.
- No rule owns did-you-mean; the behaviour is documented at `crates/nvs-hir/src/members.rs:1354` and
  pinned by the conformance case, since this goal opens no ADR number.
