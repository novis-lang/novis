# Handoff

## State

**Goal 19 — a class a string names, at one contract. Stage 2 is closed; stage 3's item 2 is on
disk, and its items 1, 3 and 4 are the next group.**

`nvs_stdlib::registry::implements_parses` is the structural predicate beside `implements_comparable`
(`crates/nvs-stdlib/src/registry.rs:2566`): a static `parse(Text(Qual::Neutral)) -> Instance(self)`
and a static `tryParse(Text(Qual::Neutral)) -> Nullable(Instance(self))`, no defaults, which is
`rule:expressions/try-parse`'s three conditions written once. `crates/nvs-types/src/core_lib.rs:86`
seeds the edge from it, two lines below the `Comparable` seed.

**`Core\Uuid` did change, and stage 3 item 2's "not one line" sentence is wrong about the tree.**
Its two rows were `CoreTy::Str`, which refuses a `tainted` argument — and `Qual::Contagious` refuses
one too wherever the answer is an object, which is the playbook bullet this session added. So the
predicate demands `Qual::Neutral` and `Core\Uuid` now carries it, out of `registry.rs`'s
`UNCLASSIFIED` ratchet in the same edit. The roster is `Core\Uuid` alone: `Core\Uri` keeps
`Contagious`, because its components come back out as plain `string`s and a `Neutral` parse there
would launder attacker text through `scheme()` and `path()` — a decision, not an oversight, and
asserted as one in `the_parses_roster_is_the_classes_carrying_the_whole_pair`.

The goal's one record is still unwritten; stage 5 schedules its prose half, and it now owes this
`Core\Uuid` reclassification and the `Core\Uri` exclusion in its `changes` block alongside
`rule:core-api/reserved-namespace` and stage 2's `E0404`.

**Pack gap, now closed:** `[context] rules` named neither
`security/unclassified-parameter-refuses-tainted` nor `security/taint-propagation`, and both decide
what this stage's predicate may require; this session derived them from
`crates/nvs-types/src/expr/quals.rs` instead. Both are added to the base manifest.

## Next group

**Stage 3: the four surfaces read the predicate instead of the name** — one file set:
`crates/nvs-types/src/commands.rs`, `crates/nvs-types/src/routes.rs`.

- [ ] **`converts_from_string`'s class arm becomes the predicate** —
      `crates/nvs-types/src/commands.rs:789` is the `QName::parse(r"Core\Uuid")` comparison inside
      `converts_from_string` (`:771`), and `crates/nvs-types/src/commands.rs:241` is the same
      comparison choosing `ArgConv::Uuid`. Both ask "does this class implement `Parses`" of the
      signature table `crate::expr::operators` asks `Comparable` of.
      `rule:expressions/try-parse` is the contract; `rule:classes/comparable` is the precedent for
      reaching the table from here.
- [ ] **The three diagnostics stop reciting the roster** — `crates/nvs-types/src/commands.rs:755`,
      `crates/nvs-types/src/routes.rs:1745` and `crates/nvs-types/src/routes.rs:1831` each name the
      closed set and then `Core\Uuid`; they name the closed set and then "a class implementing
      `Parses`". `rule:routing/a-capture-narrows-to-a-closed-set` owns what the set is.
- [ ] **The two doc comments generalize past the one class** —
      `crates/nvs-types/src/routes.rs:497` and `crates/nvs-types/src/routes.rs:1758` use `Core\Uuid`
      as the worked example of "converts, but narrows nothing", which stays true of every `Parses`
      class. `rule:routing/a-query-parameter-is-declared-like-a-capture` is the surface it is
      written about.

## Backlog

- Stage 4's two runtime arms — `crates/nvs-runtime/src/routes.rs:89` and
  `crates/nvs-runtime/src/commands.rs:57`; `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5's `examples/parses.nvs`, which is the acceptance check failing today;
  `docs/agent/loop-goal.md` § *Stage 5*.
- The goal's one record, next free 0160 — `docs/agent/loop-goal.md` § *Stage 5*.
- Stage 3 item 2's "not one line of `Core\Uuid` changes" is stale — `docs/agent/loop-goal.md:97`.
- An inherited `Parses::tryParse` still cannot be called from source; the refusal is the safe half
  and the repair is body-first — `docs/agent/playbook.md`.
