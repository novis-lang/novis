# Handoff

## State

**Goal 28 — the gaps a past milestone left and no goal claimed — has just started; nothing of it has
landed yet.** Goal 27's whole list is this goal's Stage 1 floor.

Five gaps, each recorded in a module doc, each real, none owned by any `[[goal]]` before this entry.
**Four of the five are one gap wearing different clothes**: a member that needs an options bag the
registry could not spell — which is exactly the spelling [goal 18](18-input-shapes.md) lands
(`{name?: T}` on `Ty::Shape`, and `rule:core-api/shape-parameter`'s `CoreTy` shape parameter). This goal is that
follow-through.

The other two are the decisions the user took when the unowned list was drawn up: **`array<T>` widens
to accept a covariant read**, and the panic hook `rule:errors/helper-abi` has wanted since M5 removed its
blocker.

## Next group

**Stage 2: the options bag** — one file set: `crates/nvs-stdlib/src/uri.rs`,
`crates/nvs-stdlib/src/queue.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/core_lib.rs`.

- [ ] **`Core\Uri::with` gains a removal spelling** — `uri.rs` gap 1 names the fix exactly: today an
      omitted option and a written `null` arrive as the same `Tag::Null`, so the option types are
      `string` rather than `?string`. With an optional field they are distinguishable. **Not** an
      `""`-means-remove rule: `""` is already an empty query and `query()` reports it as distinct from
      `null`.
- [ ] **`Core\Queue`'s `limits` and `grants` are declared** — § 1's `{…}` parameters, the shape the
      registry could not spell. `queue.rs` gap 1 says this is the same blocker `Core\Db::open` waits on
      and that "the two lift together"; goal 21 owns the `Core\Db` half, so check the two spellings
      against each other.
- [ ] **`Core\Queue`'s `$args` refuses a `secret`** — gap 2. `CoreTy::Mixed` carries no qualifier, so
      this needed a spelling rather than a line. A queued row is written to a database and read back by
      another process, which is a sink by every test ADR 0033 applies.

## Backlog

- **Stage 3 (`array<T>` variance)** is `nvs_types::expr::assign`'s `is_assignable` admitting an
  element-covariant array. **Decided by the user; do not re-argue it.** Sound because an Novis array is
  copy-on-write — an element-covariant *read* cannot be aliased into an unsound write, since a callee
  that writes gets its own copy. It accepts strictly more programs and breaks none, so no migration and
  no diagnostic. The negative proof matters as much as the positive one.
- **Stage 4 (the two small ones)**: the panic hook routes a served request's panic into
  `Ctx::write_log_record` with its request id — the CLI default hook's stderr stays right, and the hook
  is presentation, never containment, so nothing about it lets a panic be recovered. `[limits]
  max_output` bounds a capture at `Core\Process` **and** `Core\IO::read`, which the module doc says the
  same signature closes — both or neither.
- **Stage 5** rewrites `carried-gaps.md` § *Unowned* to what survives. One entry is expected to: ADR
  0116's optional in-flight cycle collector, which is an **open decision rather than an unclosed gap**
  and stays visible for exactly that reason.
