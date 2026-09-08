# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 4 is closed on the command side, and the
route side is decided but not written.** The tree is green (`verify.py`, 8 of 8).

**A command argument now converts through its class's own `parse`.**
`nvs_runtime::commands::ArgConv::Parses(String)` carries the label, `nvs-stdlib`'s `parse_each`
(`crates/nvs-stdlib/src/command.rs:636`) calls it through `nvs_runtime::call_static` — the same
route `Core\Command::run` already takes to the handler — and a throw becomes the usage page's
sentence, carrying what the class said. `Refused` (`:616`) is why: a word the class refused is a
page, an engine fault is returned unchanged. Two `-p nvs-stdlib --lib` cases pin both.

**`ArgConv::Parses`'s doc comment is the one home of where a conversion path may reach a compiled
`parse`.** Read it before touching either table. Its three findings: the reach is `call_static` and
needs an armed class table; the member is `parse` and never `tryParse` (see the playbook bullet);
and `CaptureConv` has no such arm because a match runs at the door.

**The route side is settled, by the user, and the goal file carries it.** A program's `parse` does
not run inside matching: the router narrows on the conversions it reads natively — `int`, `uint`,
`decimal`, `CaptureConv::Uuid` at `crates/nvs-runtime/src/routes.rs:418`, and a closed set — where a
refused segment is no match and then a `404`, and a capture typed as any other `Parses` class matches
on shape and converts at the binding site, where a segment the class refuses is a `400`. The reason
is that matching runs at the door with no program installed
(`crates/nvs-server/src/route.rs:85` holds no `Ctx`, `crates/nvs-cli/src/runner.rs:504` matches
before `unit.install_in(ctx)`), so arming a class table ahead of it would put an implementor's
`parse` over every request URL ahead of anything that rate-limits it — the priority-1 objection
`rule:routing/a-capture-narrows-to-a-closed-set` already makes to a regex, which a `parse` body
exceeds. `docs/agent/goals/19-parses.md` § *Standing decisions* is the home of it, including what it
costs; `crates/nvs-cli/src/main.rs:1413`'s `Core\Uuid` name arm stays, because it answers what the
router reads and not what may stand in a capture.

**What the settlement leaves owing**, and none of it is in the next group: the binding site itself
(`crates/nvs-runtime/src/routes.rs` gap 3), the class-typed exception in
`rule:security/route-capture-is-laundered-by-its-type` and `rule:routing/a-bad-query-value-is-a-400`,
and ADR 0160 — the rules and the record being one commit, and that commit being the one that writes
the binding site. `docs/novis.md:14048`'s "a path whose capture will not convert is claimed by
nobody" is a `Core\Router::methodsFor` sentence that goes stale the same day.

## Next group

**Stage 5: the proofs, the half the settlement does not touch** — one file set:
`crates/nvs-cli/src/openapi.rs`, `crates/nvs-cli/tests/openapi.rs`.

- [ ] **A `Parses` capture's schema is a bare `{"type": "string"}`** —
      `crates/nvs-cli/src/openapi.rs:372` is the `Core\Uuid` arm and `:356` the function; every
      other implementor answers the same string with no `format`, which is the goal's standing
      decision that a named format is a documentation hint over the schema and not a conversion
      rule. `rule:attributes/api-adds-and-cannot-contradict` is what the document owes.
- [ ] **The two named checks** — `crates/nvs-cli/tests/openapi.rs:502` is where they append:
      `a_parses_capture_is_a_string_schema` and `core_uuid_keeps_its_named_format_over_that_schema`,
      spelled exactly as `docs/agent/loop-goal.toml:5936` names them. The second is the *agreement*
      shape: the
      one class with a format keeps it while the roster around it widens.

## Backlog

- The blocked decision above — its home is the goal's one record, `docs/decisions/0160.md`.
- `examples/parses.nvs` (`loop-goal.toml:5919`) — blocked on the same answer; its fourth `want` line
  is the one that moves under (b).
- ADR 0160, the goal's one record — owes the settlement, stage 3's `ArgConv::Uuid` →
  `ArgConv::Parses` widening, the `Core\Uuid` reclassification, the `Core\Uri` exclusion,
  `rule:core-api/reserved-namespace` and stage 2's `E0404`.
- The route-side conversion site, once the answer lands — `crates/nvs-runtime/src/routes.rs` gap 3.
- A subset of an enum's cases is still `ArgConv::Unconverted` — `crates/nvs-runtime/src/commands.rs`
  gap 1, untouched by this goal.
