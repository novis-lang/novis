# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `nvs-cli`, `nvs-syntax`,
`nvs-hir` and `nvs-test`.** The roster is 52 tags, every one resolving to a live goal, a milestone
that is not `done`, or an `unowned` with its reason written; `python tools/owners.py --check
--reasons` is green. 99 items still name nobody, and they are in three crates only: `nvs-stdlib` 77,
`nvs-db` 14, `nvs-runtime` 8.

**Four of `nvs-syntax`'s six items were not gaps.** `var` in a class body is parsed
(`crates/nvs-syntax/src/parser/decl.rs:663` redirects to `parse_class_body_var`) and an enum case
takes a keyword spelling (`finish_enum_case` goes through `parse_decl_name`, and `is_name_segment`
at `crates/nvs-syntax/src/parser/ty.rs:872` is `Ident | Keyword(_)`), so both items are gone. A
grouped import is a *refused* construct rather than an unbuilt one — `recover_use_group` eats the
`\{...}` and reports `E_IMPORT_GROUP_UNSUPPORTED` with the supported spelling as its help — so it
moved to that crate's § *Deliberately rejected*. `use function`/`use const` stayed behind alone:
they reach a generic parse error today, and what a real corpus needs there is the refusal
`rule:classes/no-free-functions-or-constants` implies, not the feature.

**`nvs-hir` records no gap of its own now.** Its crate-level block named four edges: three had landed
in `nvs-types` (`AliasTable`'s consumer is `crates/nvs-types/src/lib.rs:345`, member visibility is
`crates/nvs-types/src/expr/members.rs:1570`), and trait-use flattening is not a gap at all, because
`rule:classes/no-traits` leaves the language no `trait` to flatten. The fourth — `requires` reading
only a literal path — is `rule:statements/require-is-the-only-inclusion-construct`'s own specified
behaviour, and `crates/nvs-hir/src/requires.rs:69` writes it out in full.

**The gate has a hole, and it is not one crate's** — the new playbook bullet under *Tooling*.
`owners.py` counts a `# Known gaps` heading; `requires.rs:69`'s five bullets sit under a
`**Known gaps:**` bold run, so the roster has never counted them and closing stage 3 will not.

**`carried-gaps.md` § *Owned* lost its `doc-comments` row.** Its four gaps are now two closures, one
decision and one `M1` tag living with the gap, which is where this goal puts an owner. § *Unowned*
is fourteen entries: the new one is that a local declaration cannot be typed with a bare inline
shape type, where what has to be decided is whether the grammar buys lookahead past a matched
`{...}` or `rule:types/shape-type`'s alias example is the answer for a local.

**`python tools/verify.py`: 9 of 9 green** — 3838 tests, both `.nvst` trees, the reference, the
extension and clippy.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-runtime`'s eight
items, which are the last outside `nvs-stdlib` and `nvs-db`. The kinds are the goal's § *Standing
decisions*, the three owner kinds are `python tools/owners.py --help`, and `--untagged` is the
worklist.

- [ ] **Tag `crates/nvs-runtime/src/routes.rs:74`'s two items** — a linear scan where
      `rule:routing/path-grammar` names a trie, and at `crates/nvs-runtime/src/routes.rs:79` a
      reader that answers the name and the captures but never the row. A shape a rule already
      specifies is scheduled work, so weigh a milestone tag before `unowned`.
- [ ] **Tag `crates/nvs-runtime/src/graph.rs:61`'s two items** — a closure recognized by its class's
      `invoke` method, and at `crates/nvs-runtime/src/graph.rs:66` a `decode` that resolves a class
      through the program's table only. `rule:security/isolate-values-cross-by-copy` is the contract
      both sit under.
- [ ] **Tag the four singles** — `crates/nvs-runtime/src/decimal.rs:45` (a 128-bit intermediate
      throws rather than rounding) and `crates/nvs-runtime/src/decimal.rs:52` (nothing inlines),
      `crates/nvs-runtime/src/array.rs:214` (no interned element-type descriptor), and
      `crates/nvs-runtime/src/commands.rs:41` (`ArgConv::Unconverted` over a subset of an enum's
      cases).

## Backlog

- `owners.py` reads a heading and not a `**Known gaps:**` bold run, so `crates/nvs-hir/src/requires.rs:69`'s
  five bullets are invisible to the gate — `tools/owners.py`, goal `gap-owners`.
- `crates/nvs-db`'s 14 items across `catalog.rs`, `ddl.rs`, `matrix.rs` and `schema.rs` — the group
  after `nvs-runtime`.
- `crates/nvs-stdlib`'s 77 items are the bulk of stage 3 and will not fit in one session.
- This clone has no git hooks; `git config core.hooksPath tools/git-hooks` is what `verify.py` asks
  for on every run — `docs/agent/conventions.md` § *A commit message*.
