# Handoff

## State

**ADR 0064 § 5's one parser is landed, and ADR 0104 § 3's bound with it.**
`crates/nvs-config/src/value.rs` is the parser: `Quantity::parse(key, unit, setting)` reads a
size, a duration, a count, a ratio or the `false` that removes a ceiling, and `Quantity::within`
is the comparison. `crates/nvs-config/src/app.rs:137`'s `bound` is its first caller — it runs in
`resolve` right after `canonicalize`, and refuses `E0610` for an `[app.limits]` value or an
`[app.limits.hard]` ceiling above the global `[limits.hard]`. 82 tests in the crate, 10 new in
`tests/value.rs` and 4 in `tests/app.rs`.

**The parse is directed by the unit, and the unit comes from the key.** `m` is mega under a size
and minutes under a duration, and a bare `600` is bytes under one and seconds under the other, so
nothing in the spelling can decide it: `value::unit_of` decides once for both the boot path and
`Core\Config::set`. It is keyed on the key's **last segment** inside a limits block, because one
limit is written in five places — `[limits]`, `[limits.hard]`, `[app.limits]`,
`[app.limits.hard]` and the bare `memory` a `set` names — and a ceiling that parsed differently
from the value it bounds would compare two different quantities. The module doc owns the rest;
adding a sixth limit is one row in `unit_of` and one in `app::ceilings`.

**Two things § 3 does not state are deliberately unchecked**, recorded in `bound`'s own doc: a
per-app default above the block's own lowered ceiling, and the same shape globally. Both are
incoherent rather than unsafe — the value in force is still bounded by the ceiling the request is
held to — so refusing them would be inventing a rule.

**`origin_note` has one home now**, `crates/nvs-config/src/resolve.rs:177`, `pub(crate)` beside
`Origin`. It was copied in `app.rs` and `secret.rs` and this slice would have been the third.

**The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`.** Still
an open item and not a regression: the members have never existed, and they are now the only
thing between this goal and that check. Nothing else blocks.

## Next group

**`Core\Config`'s four members — ADR 0064 § 5's API, and the request-local overlay under them.**
One file set: a new `crates/nvs-stdlib/src/config.rs`, `crates/nvs-stdlib/src/registry.rs:984`
(`CLASSES`), `crates/nvs-runtime/src/ctx.rs:246` (`Ctx`), and `crates/nvs-stdlib/Cargo.toml`.
**Read `AGENTS.md`'s *A `Core` member — the five edits* before starting**; `examples/config.nvs`
is the program the driver's check runs and its four output lines are frozen in `loop-goal.toml`.

- [ ] **A request reaches its snapshot.** No crate outside `nvs-config` depends on it today —
      that is the first thing to change, and where the `Arc<Snapshot>` a request clones
      (`crates/nvs-config/src/snapshot.rs:263`'s `Current::load`) is held is the decision:
      `Ctx` at `crates/nvs-runtime/src/ctx.rs:246` is cold-half material, and ADR 0078 § 1 says
      the clone happens once at request start, never per read.
- [ ] **`Core\Config::get` and `all` read it.** ADR 0064 § 5 — values cross as `string` in both
      directions, so a `Setting` renders back the way it was written
      (`crates/nvs-config/src/value.rs`'s `as_written` is that, `pub(crate)` today).
- [ ] **`set` and `restore` write the copy-on-write overlay.** ADR 0005 through
      `crates/nvs-config/src/directive.rs:128`'s `lookup` for the class, then
      `crates/nvs-config/src/value.rs:217`'s `within_ceiling` for the ceiling — the same call
      `bound` makes, which is the whole reason the parser is one implementation. A refused set
      returns `false` and leaves the value in place; it does not throw (goal § *Standing
      decisions*).

## Backlog

- Item 18's `Core\Secret::reveal()` is not in the registry — `crates/nvs-stdlib/src/registry.rs`.
- `Live::admit`'s same-class check asks the answer, not the argument — `nvs-runtime/src/graph.rs`.
- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- A `[limits]` default above its own `[limits.hard]` is unchecked, globally and per-app; ADR 0005
  states no rule for it — decide and record before `Core\Config::set` reads the pair.
- ADR 0104 has no `Validated by:` field though `tests/app.rs` now holds §§ 1-3's claims.
- `orient.py` reported `[context] modules` pattern `crates/nvs-host/src/budget.rs` matching no
  module — the file moved or the glob is wrong, in `docs/agent/loop-goal.toml`.
