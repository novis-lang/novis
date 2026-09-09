# Handoff

## State

**Goal 40 — an agent learns Novis from the binary, in three calls. Stages 0 and 2 are landed;
stage 1 is goal 39's floor and carries.** Nothing is blocked, and no design question is open — the
goal's § *Standing decisions* pre-authorizes every call the remaining stages reach.

On disk: `nvs agent index|find|show`, rendering `crate::meta::document()` and holding nothing, and a
`capabilities` roster as a seventh top-level key of `nvs meta --json`. `nvs agent primer` is stage 3
and is not built, so `rule:tooling/an-agent-asks-the-binary` stays `designed` while
`rule:tooling/the-index-is-one-line-per-member` is now `shipped`.

The stage 0 audit's verdicts, so no session re-runs it: `nvs doc <entry> --out <dir>` and
`nvs meta --json <entry>`'s `program` key both ship and are guarded, and both rules are flipped.
`registry::CAPABILITIES` had exactly one reader — `crates/nvs-stdlib/tests/capability.rs` — so
`rule:security/capability-declaration-is-one-table`'s sentence naming three consumers was false on
two; it now names the closure test and the metadata command, and `tools/reference.py` still prints no
capability beside a member's card (backlog below).

The `[context]` manifest had no gap: stage 2's overlay already names every rule this needed. The
handoff it was read against named stage 0 while the group's second slice was stage 2, so the pack
opened one overlay short — name the *later* stage when a group spans two.

## Next group

**Stage 3: the primer, generated** — one file set: `crates/nvs-cli/src/agent.rs`,
`crates/nvs-cli/src/main.rs` and `tools/reference.py`, reading the chapters under `docs/spec/`.
`docs/novis.md` is generated from those and is not where a marker goes.

- [ ] **Mark the chapter sections the primer lifts**, in `docs/spec/`, with
      `tools/reference.py:106`'s `parse_front` and `tools/reference.py:122`'s `load_chapters` as the
      readers. `rule:tooling/a-primer-claim-is-executed` fixes what it carries — the lookup
      protocol, one complete worked program, the capability model with the smallest `nvs.toml` that
      grants a file read, the refusal table, the chapter map. A section is lifted because it is
      marked and for no other reason: `an_unmarked_chapter_section_is_not_lifted_into_the_primer` is
      that guard, and the primer is never hand-written.
- [ ] **`crates/nvs-cli/src/agent.rs:156`'s `index` gains `primer` beside it**, and
      `crates/nvs-cli/src/main.rs:550`'s `AgentCommand` gains a `Primer` variant. It assembles the
      marked sections and the registry at the call and caches nothing, like the three verbs already
      there. Landing it is what flips `rule:tooling/an-agent-asks-the-binary` to `shipped`.
- [ ] **`python tools/reference.py --primer --check`**, whose flags go beside the others at
      `tools/reference.py:712`'s `main` — every spelling the primer states as refused is fed to
      `nvs check` and must be refused, and every example in it is run and must print what the primer
      says (`rule:tooling/a-primer-claim-is-executed`). This is a `command` check in the goal file,
      not a `#[test]`, so it fails on the tool's own exit status.

## Backlog

- `tools/reference.py` prints no capability beside a member's card though `nvs meta --json` now
  emits the roster to join — `docs/rules/security/capability-declaration-is-one-table.md` is the
  rule that used to claim it did.
- `nvs agent show` renders no card for a *class*; `show Core\IO` answers with the nearest matches,
  which is that class's members. No rule asks for one — `rule:tooling/an-agent-asks-the-binary`.
- Stage 4's two diagnostics: the runtime capability refusal's wording,
  `crates/nvs-runtime/src/capability.rs`, and `E0309` for a member that does not resolve.
- Stage 5's adapters — `rule:tooling/an-adapter-carries-protocol-and-never-language`, and
  `nvs agent init --embed` is decided against and not to be revisited.
- Static capability checking stays a carried gap owned elsewhere — [carried-gaps.md](carried-gaps.md).
