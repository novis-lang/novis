# Handoff

## State

**Goal 19 — stage 3 is closed.** All eight names its two `cargo-named` checks list exist and pass:
six in `crates/nvs-types/tests/` (`cargo test -p nvs-types --test routes --test commands`) and two
in `crates/nvs-stdlib/src/registry.rs`'s test module. The predicate is asserted from both ends — a
user class reaches a capture, a `#[Query]` and a command argument by implementing `Parses`, a class
without the interface is refused with the fix named in the help, and `Core\Uuid` reaches the same
roster through the same bit rather than through an arm matching its name.

**Stage 5's reference pages are done** (`docs/reference/lang/50-classes.md`,
`docs/reference/lang/90-attributes.md`, `docs/novis.md` regenerated). **Stage 4 is what is left
before them**, and part of it is already on disk: `crates/nvs-runtime/src/routes.rs:450` converts a
`Parses` capture and `:456` a `decimal` one. `crates/nvs-runtime/src/commands.rs:158`'s
`ArgConv::Unconverted` is that module's gap 1 and is the command side's remaining half.

**One stage-4 check name states the answer this goal's § *Standing decisions* settled the other
way** — see the first item below. It is a name to amend, not work to do.

**Still open and not this stage's:** `tryParse` is unreachable on a user implementor, nothing binds
a `#[Query]` parameter, and a class-typed `#[Query]` cannot carry a default (`E0451` wants a literal
of the declared type).

## Next group

**Stage 4: the runtime arm — one file set: `docs/agent/loop-goal.toml`,
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-runtime/src/commands.rs`.** The five names are
`docs/agent/loop-goal.toml:5904`'s `cargo-named` list, verbatim once the first item has amended one
of them.

- [ ] **The check name that contradicts the goal** — `docs/agent/loop-goal.toml:5905` reads
      `a_segment_parse_refuses_is_no_match_rather_than_a_matched_bad_value`, and
      `rule:security/route-capture-is-laundered-by-its-type` says the opposite for this class of
      capture: a `Parses` class matches on **shape** and its refusal is a `400` over a route that
      did match, because the matcher runs at the door with no program installed. Rename it to what
      the rule says, then copy `docs/agent/loop-goal.toml` over `docs/agent/goals/19-parses.toml` —
      they are byte-identical by construction.
- [ ] **The route half** — `a_parses_capture_converts_the_segment_before_the_handler_is_reached`,
      the renamed refusal test and `a_decimal_capture_is_converted_and_no_longer_unconverted`, in
      the test module at `crates/nvs-runtime/src/routes.rs:655`. The arms they read are
      `crates/nvs-runtime/src/routes.rs:450` and `:456`; the module doc at
      `crates/nvs-runtime/src/routes.rs:38` owns why this walk performs no `parse`.
- [ ] **The command half** — `a_parses_command_argument_is_converted_and_a_refusal_is_a_usage_error`
      and `a_decimal_command_argument_is_converted_and_no_longer_unconverted`, over
      `crates/nvs-runtime/src/commands.rs:158`'s `ArgConv::Unconverted`, which is that module's own
      gap 1 and the only half of stage 4 that is a change to the conversion rather than a test over
      one. `crates/nvs-runtime/src/commands.rs:119` is where the two sides' different failure
      directions are already written down.

## Backlog

- Stage 4's `nvs-suite` check names three `.nvst` cases that exist on disk; run them before writing
  runtime tests — `docs/agent/loop-goal.toml:5911`.
- Stage 5's `examples/parses.nvs` and `examples/parses.nvsr` do not exist —
  `docs/agent/loop-goal.toml:5926`.
- Stage 5's two `-p nvs-cli` OpenAPI tests — `docs/agent/loop-goal.toml:5947`.
- `tryParse` is unreachable on a user implementor — `docs/agent/playbook.md` has the shape.
- Nothing binds a `#[Query]` parameter at run time — stage 5's `exact` check reads its two `tag`
  lines by hand for exactly this reason.
