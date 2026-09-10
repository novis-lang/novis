# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `nvs-runtime`**, after `nvs-cli`,
`nvs-syntax`, `nvs-hir` and `nvs-test`. 91 items still name nobody, and they are in two crates only:
`nvs-stdlib` 77 and `nvs-db` 14. `python tools/owners.py --check --reasons` stays green.

**`nvs-runtime`'s eight went six `unowned`, one `M12`, one deleted.** The `M12` is `decimal.rs` gap 2 —
"nothing inlines" is the optimising JIT tier's own scope, and `docs/plan/m12.md:3` names inlining and
the unboxed fast paths outright. The deletion is `routes.rs`' second item: the match crossing as the
name and the captures and never the row is a refusal `rule:routing/matching-is-not-dispatching` owns,
with the day it is re-argued written into it, so it moved into § *What matching is, and what it is
deliberately not* as prose.

**Only goals 30–44 are live**, and that is what decides a milestone tag: everything from `core-depth`
to `xml-tree` is retired, so M4S, M5 and M8's database half are *carried* and their leftovers are
`unowned`, not theirs. The new playbook bullet under *Tooling* is the tell and the check.

**`carried-gaps.md` § *Unowned* is twenty entries.** The six added name a decision each: whether a
trie is ever the request path's problem, whether a closure earns a bit on its class descriptor,
whether `nvs_stdlib::instance`'s table is installed on `Ctx`, whether every division pays a 192-bit
fold, whether an array header carries the element-type word before a reader needs it, and which end
closes a command argument typed as a subset of an enum's cases.

**`python tools/verify.py`: 9 of 9 green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-db`'s fourteen items,
which are the last outside `nvs-stdlib`. The owner kinds are the goal's § *Standing decisions* and
`python tools/owners.py --help`; `--untagged` is the worklist and `--check --untagged-is-an-error
--reasons` is the gate. Goal `database` is retired, so M8's database half is carried and a bare `M8`
tag is wrong for anything it left behind — read `docs/plan/m8.md`'s database paragraph before reaching
for one.

- [ ] **Tag `crates/nvs-db/src/ddl.rs:47`'s six items** — the portability gaps of the DDL writer, at
      `crates/nvs-db/src/ddl.rs:47`, `:56`, `:60`, `:65`, `:72` and `:77`. Four of the six are one
      engine's own limit rather than an unbuilt shape, so weigh *decision* before `unowned`: the goal's
      § *Standing decisions* resolves that ambiguity toward the decision and out of the block.
- [ ] **Tag `crates/nvs-db/src/catalog.rs:55`'s five items** — the introspection reader against spec
      § 11's vocabulary, at `crates/nvs-db/src/catalog.rs:55`, `:65`, `:70`, `:76` and `:86`. `:70`
      says § 5 owes the other half, so check whether that half landed before tagging it at all.
- [ ] **Tag the three singles** — `crates/nvs-db/src/schema.rs:46` and `:59` (§ 11's exclusions, which
      read as decisions rather than gaps) and `crates/nvs-db/src/matrix.rs:43`, whose socket leg is
      `rule:core-classes/db-unix-socket-path`'s transport asserted against no real server: it waits on
      a container's socket directory bind-mounted onto the host, which is a `[[check]]` precondition
      before it is anyone's feature.

## Backlog

- `nvs-stdlib`'s 77 items are the whole of what is left after `nvs-db` — `python tools/owners.py
  --untagged` groups them by file, and `db/mod.rs` alone is eight.
- The gate still cannot see a `**Known gaps:**` bold run — `crates/nvs-hir/src/requires.rs:69`'s five
  bullets — and closing stage 3 will not close that hole; the playbook bullet under *Tooling* is its
  home.
- Some of what is left is not a gap: `crates/nvs-stdlib/src/db/mod.rs:251` says outright that its
  missing `{chunk?: uint}` is a refusal, and the goal's § *Standing decisions* moves that kind out of
  the block rather than tagging it. Expect the `nvs-stdlib` pass to delete as well as tag.
