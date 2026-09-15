# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and **stages 3 to 7 are complete**. Stage 8 is open: its `nvs-suite` check wants three `.nvst` cases
and one of them is on disk. Nothing is blocked.

**An enum capture matches and hands its case over, and the link half does not exist yet.**
`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name` is `designed` still,
and its fragment now says which half is built: `crates/nvs-types/src/routes.rs`'s `enum_capture`
decides the spelling while compiling — the written backing value where every admitted case wrote one,
the case name where any counted — `RouteParam::cases` carries the spelling and the value it becomes
to the door, and `nvs_runtime::routes::CaptureConv::Enum` converts on it. A segment naming no
admitted case is no match, like a failed `int`.

**A case arrives as its backing integer and needs no union member.** `CAPTURE` in
`crates/nvs-stdlib/src/router.rs:558` is unchanged: a case is indistinguishable from its integer by
the time it is a value (`rule:enums/no-class-machinery`), so the match hands over `Param::Int` or
`Param::Uint` and `Core\Router\Match::param` answers the case rather than the segment.

**`EnumInfo::written` is where "was this value written" lives**, because a counted value is an
ordinary integer everywhere after the declaration is read. A `Core` enum's is recovered from the run
of values `registry::ENUMS` states for every case: one that is not what auto-increment would have
handed it wrote its own.

## Next group

**Stage 8: the routes, the link half** — one file set: `crates/nvs-types/src/links.rs`,
`crates/nvs-types/src/routes.rs` and `crates/nvs-stdlib/src/router.rs`.

- [ ] **`url` writes an enum case's spelling and refuses one outside the subset** — the two halves
      move together or a correct link is refused: `RouteParam::allowed` is still filled from
      `closed_set` alone (`crates/nvs-types/src/routes.rs:1723`) and `segment_text` folds a case
      argument to its backing integer (`crates/nvs-types/src/links.rs:203`), which is the admitted
      spelling for a value-spelled subset and not for a name-spelled one. Decide first whether the
      link is rendered while compiling — `crates/nvs-types/src/links.rs:19` says the call is replaced
      with its answer — or substituted at run time in
      `crates/nvs-stdlib/src/router.rs:834`'s `substitute`, which today is handed the template and
      not the row. `rule:routing/link-name-and-params-are-checked`,
      `rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`. That closes
      `crates/nvs-types/src/links.rs:53`'s gap 2 and the rule's status goes to `shipped`.
      Case: `tests/conformance/reject/router-url-refuses-an-enum-case-outside-the-captured-subset.nvst`.
- [ ] **`url` prepends the mount prefix the door stripped** — `crates/nvs-stdlib/src/router.rs:65`'s
      gap 2, owned by this goal: an inbound request carries `Ctx::inbound`'s `mount_prefix` and no
      member here asks for it, so a link is written from the mount root out.
      `crates/nvs-stdlib/src/router.rs:834` is the substitution and
      `rule:routing/link-carries-the-mount-prefix` is the rule. Settle first how a synthetic request
      gets a non-empty prefix — `Core\Test::request` (`crates/nvs-stdlib/src/test.rs:1902`'s
      `described`) builds an `InboundSpec` and the check wants a `.nvst`, not a `-p nvs-cli` test.
      Case: `tests/conformance/core/router-url-prepends-the-mount-prefix-the-door-stripped.nvst`.

## Backlog

- Stage 8's second check is `-p nvs-cli`: a served request receives its mount's resolved origin, and
  a mount calling `urlAbsolute` with none refuses the boot — `rule:routing/an-origin-is-per-mount-and-checked-at-boot`.
- The `#[Query]` half of the enum spelling is unbuilt: a value still arrives as its text, though
  `RouteParam::cases` is filled for a query key too (`crates/nvs-types/src/routes.rs:1852`).
- `docs/agent/loop-goal.toml`'s `[context] rules` named neither
  `routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name` nor `enums/declaration`,
  and this item is about both; `modules` named no `crates/nvs-types/**`, which is where the work was.
