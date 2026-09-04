# Handoff

## State

**Goal 6, M7. ADR 0086 § 6's argument matcher converts every type on its roster but two.** A
`decimal` and a `Core\Uuid` command argument convert, and a word that is not one is a *usage*
error — a command line is input, so it is never a throw. Both read the runtime's own parse rather
than a second one written beside the matcher: `nvs_runtime::decimal` and `nvs_runtime::uuid`, the
same homes `nvs_runtime::routes::convert` reads one table along, so a word a command line supplies
and a segment a route matches are admitted by one grammar.

**`nvs_runtime::commands`' gap 1 is now two, not four: an enum and a union of literal types.** Both
need the same thing and it is not the algorithm — a **closed set carried on the row**, which
`CommandArg` has no field for. That set already exists one table along as
`nvs_runtime::routes::CaptureConv::OneOf`, computed by `nvs_types::routes::closed_set`; that
module doc is the home of the gap's statement.

**A `decimal` or `Core\Uuid` argument still cannot carry a default, and that is `crate::defaults`'
gap rather than this one's** — a non-literal default is refused before it folds, so an argument at
either type stays required. `nvs_types::commands::default_text`'s doc is the home of that reading.

**The driver's failed acceptance check is unchanged and still an open item, not a regression**:
neither half of `the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again`
exists — `nvs-server` neither enforces ADR 0096 § 4's CSRF check nor emits ADR 0076 § 1's `route`
label off the match ADR 0102 § 1 already makes at the door. It is Backlog below, not this group.

**`orient.py` did not print ADR 0086 § 6 itself.** Every anchor cited it and the pack had 0102's
sections instead; the work was done off the code's own citations. `[context] adrs` wants
`0086:6`.

## Next group

**The rest of `crate::commands`' gap 1 — the two conversions that need a closed set on the row.**
The file set is the one this session already had open: `crates/nvs-types/src/commands.rs`,
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-cli/src/main.rs` and
`crates/nvs-stdlib/src/command.rs`, plus the two route-side files that hold the shape to copy.

- [ ] **`CommandArg` carries a closed set, and an enum argument converts** (ADR 0086 § 6) — the
      variant to add is beside `crates/nvs-runtime/src/commands.rs:57`'s `Decimal`, the shape to
      copy is `crates/nvs-runtime/src/routes.rs:106`'s `OneOf(Vec<String>)`, the computation to
      reuse is `crates/nvs-types/src/routes.rs:1738`'s `closed_set`, the choice is
      `crates/nvs-types/src/commands.rs:185`, the crossing is `crates/nvs-cli/src/main.rs:784`,
      and the arm that turns text into a value is `crates/nvs-stdlib/src/command.rs:513`. The
      value an enum case becomes is the question `nvs_runtime::routes` answers with
      `Param::Text`, and a command argument declared at an enum wants the *case*, so this slice
      decides that and records it in `crates/nvs-runtime/src/commands.rs:57`'s own doc.
- [ ] **A union of literal types converts** (ADR 0086 § 6) — the same six anchors and no second
      variant: `closed_set` already answers a union at
      `crates/nvs-types/src/routes.rs:1738`, and § 3 admits exactly the members
      `crates/nvs-types/src/commands.rs:699` lists.
- [ ] **The corpus follows, and the gap case has no type left to use**
      (`crates/nvs-stdlib/src/command.rs:281`) — with both conversions landed, no *compiling*
      program can reach `ArgConv::Unconverted`, so
      `tests/conformance/core/command-run-throws-for-a-parameter-no-argument-converts-into.nvst:1`
      is deleted rather than rewritten a second time, and the variant stays only because
      `conversion_of` is total. Two `.nvst` cases replace it, one per conversion, each naming the
      value a good word becomes and each with a sibling pinning the usage status for a word
      outside the set — the shape
      `tests/conformance/core/command-run-converts-a-decimal-argument-to-the-exact-number-written.nvst:1`
      already has.

## Backlog

- `the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` — neither half
  exists; ADR 0096 § 4 and ADR 0076 § 1 over ADR 0102 § 1's match, in `crates/nvs-server`.
- `docs/agent/goals/19-parses.md:116` and `:122` still say `decimal` and `Core\Uuid` are
  `Unconverted` as a capture and as an argument; both went stale this session and the last. That
  goal has not run — its own doc owns the repair.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- `nvs_runtime::routes`' three known gaps: the linear scan, the row that does not cross, and
  percent-decoding — that module's doc owns them.
