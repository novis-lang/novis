# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 4 is closed on the command side and
BLOCKED on the route side.** The tree is green (`verify.py`, 8 of 8).

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

**BLOCKED — the user has to choose, and the two sides cannot both hold.** `loop-goal.toml:5922`'s
frozen fixture wants `"a segment the class refuses did not match"`, which puts a program's `parse`
*inside* matching. But matching runs at the door: `crates/nvs-server/src/route.rs:86` holds no
context at all and `crates/nvs-cli/src/runner.rs:504` matches before `unit.install_in(ctx)`, and
arming one earlier makes an implementor's `parse` application-authored code on the request path
ahead of everything that rate-limits it — 0102 § 5's own "0075 is above the handler, not above the
match", and the priority-1 objection `rule:routing/a-capture-narrows-to-a-closed-set` already makes
to a regex, which a `parse` body exceeds. **(a)** accept that and arm the table before the match, or
**(b)** convert after the match, where a refusal is a `400` on
`rule:routing/a-query-parameter-is-declared-like-a-capture`'s reading — a change to
`rule:security/route-capture-is-laundered-by-its-type` and to one frozen `want` line. **(b) is the
safe option and the tree carries it:** the capture stays `CaptureConv::Unconverted`, converting
nothing and refusing nothing, and `crates/nvs-runtime/src/routes.rs`'s gap 3 states the question.

## Next group

**Stage 5: the proofs, the half the open question does not touch** — one file set:
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
