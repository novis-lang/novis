# Handoff

## State

**Goal 1's floor gate is green and `examples/routes.nvs` prints all five of its `want` lines.**
Two things landed.

**Item 15 in `docs/agent/loop-goal.md` owns M4's seventeen `nvs-ir` lowering refusals.** They were
never a regression: `goal-switch.py` carries the outgoing goal's `[[check]]` blocks forward as the
floor and its *unclosed items* not at all, so `every_refusal_is_a_diagnostic_or_decided` arrived in
Stage 1 without the item list that made it green. Item 15 is the inventory — seven files, every site
anchored — and it states outright that this goal does not close them; what makes standing acceptable is
`CEILING` at `crates/nvs-ir/tests/refusals.rs:66`, which holds the total at seventeen and may never
rise. `python tools/holes.py --unattributed` reads 0 and `--item 15` prints all seventeen.

**`Core\Router::urlAbsolute` answers.** `Ctx::origin` (`crates/nvs-runtime/src/ctx.rs`, the field beside
`diagnostic`) carries ADR 0102 § 6's origin, written *before* the request runs and by nothing on it,
which is what makes "configured, never sniffed" a property of the shape. `configured_origin`
(`crates/nvs-cli/src/main.rs`) resolves it from `./nvs.toml`'s `[app] origin` — ADR 0103 § 1 step 2's
location, not the entry file's directory — and the repository's own `nvs.toml` is that fixture, one key
and a comment saying why it is one key. A unit resolving none still throws at
`crates/nvs-stdlib/src/router.rs:303`, naming the route it could not build.

**`configured_origin` is not `nvs.toml`'s reader and must not grow into one.** M6's is, with ADR 0064's
syntax, ADR 0103's include tree and ownership check, and ADR 0005's registry. Its doc comment is that
rule's home.

## Next group

**One file set — `crates/nvs-types/src/links.rs`, `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-stdlib/src/uri.rs`, `tests/conformance/core/` — then one tooling slice that shares none of
it and can go last or alone.**

- [ ] **A `.nvst` case over the two link refusals.** An unknown literal name (`E0754`) and a `$params`
      covering less than the path's captures (`E0755`), both `--EXPECTF-ERROR--`; plus the positive twin
      — a `{page?}` left out of `$params` is *not* refused, because its whole segment is dropped
      (`crates/nvs-stdlib/src/router.rs:253`, the `link::OPTIONAL` arm). Both messages are written at
      `crates/nvs-types/src/links.rs:165`. ADR 0077 § 4.
- [ ] **A `$params` key naming no capture becomes a percent-encoded query string.** ADR 0102 § 6's other
      half. `substitute` at `crates/nvs-stdlib/src/router.rs:234` is where it lands — the prepared pieces
      name every capture, so what is left over in `$params` is the query — and
      `crates/nvs-stdlib/src/uri.rs`'s query builder is the implementation; do not write a second one.
      `links.rs`' gap 1 owns why the *refusal* half waits on `#[Query]`.
- [ ] **`goal-switch.py` carries the outgoing goal's unclosed items, not only its checks.** The bug item
      15's last paragraph names: a carried check whose green depends on an item list arrives without its
      basis, and every goal in `docs/agent/goals/chain.toml` otherwise inherits item 15 by hand.
      `tools/goal-switch.py:41` (`LIVE`/`MARKER`) is the insertion point and `tools/holes.py:194`
      (`items`) is the reader whose contract it has to satisfy — a one-line bold title, per the playbook.

## Backlog

- Item 15's seventeen `nvs-ir` refusals stay open by design — `docs/agent/loop-goal.md` item 15 is the
  inventory and the ratchet is the guard.
- ADR 0102 § 6 spells the fallback `[app] origin` while ADR 0104 § 1 makes `[[app]]` an array of tables
  keyed on `root`/`entry`; one of the two bodies is stale and the ADR body is the rule.
- `crates/nvs-test/src/case.rs:193` still says `nvs.toml` is not read until M6 — true of the reader,
  no longer true of the file.
- The mount prefix `Core\Router::url` prepends is still empty, and there is no `::match` — goal 6's,
  per this goal's § *Standing decisions*.
- Items 10 and 11 (every class's conformance floor to 3, every `Core` member's error paths asserted) are
  the standing fallback when a group is blocked — `python tools/gaps.py` ranks both.
- `nvs.toml`'s real reader, with ADR 0103's include tree and ownership check — M6's, and the one thing
  that retires `configured_origin`.
