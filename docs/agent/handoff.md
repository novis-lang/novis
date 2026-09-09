# Handoff

## State

**Goal `xml-tree` is reached — `python tools/loop.py --goal-only` says every acceptance check
passes.** Stage 5 landed: `Core\Html::sanitize(tainted string): Core\Html\Markup` is the class's
second launderer, the four tests its `[[check]]` names pass including the mXSS corpus, and
`examples/html-sanitize.nvs` exists, which closes the check that had failed since session 0002.

The last red check was neither: `5 registered` named `every_migration_member_is_registered`, a
shorthand no crate declares, where `spec_registry_coverage.rs` has carried
`every_migration_member_row_names_a_registered_member` — the same walk — throughout. The stage 1
floor copy of that check already carried the correction and the stage 5 copy did not; both goal
files now do. The playbook bullet for it was already written.

**The member is three steps and only the middle one holds a policy** — `parse`, `rebuilt`, `source`,
all in `crates/nvs-stdlib/src/html.rs`. The serialiser is written *here* rather than in `crate::xml`
because `rule:core-classes/html-parsing` says serialization follows the door; `crate::xml` still has
no tree serialiser of its own and writes through `Core\Xml\Writer` instead.

**The allowlist is `ELEMENTS`, 54 entries, closed and sorted** (`allowed` bisects it), with `GLOBAL`
for `dir`/`lang`/`title` and `addressable` for the three URL attributes. It grants no `class`, `id`,
`style` or `target`, and `the_allowlist_is_closed_and_is_not_configurable_by_a_caller` fails on the
day one is added by hand.

`rule:core-classes/html-sanitize` is now `shipped` with the three `.nvst` cases as its guards, and
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` already lost `§17 Core\Xml`, so stage
5's fourth item was closed before this session.

## Next group

**Follow-ups this goal did not owe** — one file set, `crates/nvs-stdlib/src/html.rs` and
`crates/nvs-stdlib/src/xml.rs`. None is a gate on anything; a goal switch discards this list, which is
the right outcome if the sweep agrees the goal is met.

- [ ] **A tree serialiser for the XML door**, so a walked `Core\Xml\Node` can be written back without
      replaying it through `Core\Xml\Writer`. `rule:core-classes/html-parsing` § *serialization
      follows the door* is what specifies the difference — refusing what the HTML one recovers from,
      an end tag on every element — and the row would sit beside `children` at
      `crates/nvs-stdlib/src/xml.rs:267`.
- [ ] **`class` on the sanitizer's allowlist, or a written reason it is not there.** The standing
      decision in `docs/agent/loop-goal.md` § *Standing decisions* says the list gains an element in
      a commit with a reason rather than by a parameter; `class` is the first one a real application
      will ask for, and the table is `crates/nvs-stdlib/src/html.rs:1273`.
- [ ] **A `Core\Xml\Node` case over a sanitized document**, asserting the two doors agree about the
      answer's tree. `rule:core-classes/html-parsing`'s one-node-family clause is what it pins, and
      the walk to copy is `crates/nvs-stdlib/src/html.rs:1853`.

## Backlog

- `Core\Metrics` and `Core\Signature` are the last two keys in
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`, owned by `unowned` and
  `signed-urls`.
- Comment nodes are dropped by `sanitize` rather than kept and escaped — recorded in `verdict`'s own
  comment, `crates/nvs-stdlib/src/html.rs`.
