# Handoff

## State

Goal `lang-attributes`, 3 of the group's **8** features complete in `python tools/dossier.py --gate
--group lang:attributes`: `an-attribute-is-a-shape-literal-attached-to-a-declaration`,
`core-api-and-the-openapi-document` and `core-command-and-core-option-the-command-table`. Nothing is
blocked. (An earlier handoff said the goal held three features; the group holds eight, and
`--gate --group lang:attributes` is the list.)

Two things the two landed features settled, so the next session does not re-derive them. **A `lang:`
feature whose perf figure is measured goes stale when `docs/reference/lang/90-attributes.md` is
edited** — that chapter is what every `lang:attributes/…` feature is "implemented at" — so a slice
that corrects the chapter re-measures with `--record-perf --only` before it wraps. And **`#[Core\Api]`
has no runtime surface at all**: `Core\Attributes::get` answers `null` for it, so its perf proof is a
`[skip]` entry in `tools/data/dossier-policy.toml` with the reason, while `#[Core\Command]` and
`#[Core\Option]` *are* readable and carry a real bench.

One bug found and not fixed here: a folded `null` read straight through `?->` throws instead of
short-circuiting. It is a lowering bug rather than a `Core\Attributes` one, its anchor is
`crates/nvs-ir/src/lower/expr.rs:1118`, and the playbook carries the workaround. It is in the
backlog below rather than in a `# Known gaps` section, because `python tools/owners.py --check`
refuses a gap that names no milestone at M9 or later and no milestone owns this.

## Next group

**Stage 2: the dossier, three features over one reference chapter** — one file set:
`docs/reference/lang/90-attributes.md`, `docs/examples/lang/attributes/`,
`tests/hostile/lang/attributes/`, `benches/members/lang/attributes/`, `tests/conformance/core/`.
All three read the chapter the same way and all three are `Core\Attributes`-shaped, so the proofs
share their idiom with the two that landed.

- [ ] **`lang:attributes/reading-attributes-back-core-attributes-get-and-all`** — owes about,
      examples, hostile, perf, tests. The members' own reference card and the structural-match rule
      are at `docs/reference/lang/90-attributes.md:92`; the registry row, the `$member` contract and
      the new `# Known gaps` item are `crates/nvs-stdlib/src/attributes.rs:1`.
      `rule:attributes/structural-retrieval` is what the written-vs-computed `$member` proof owes.
- [ ] **`lang:attributes/the-names-the-compiler-acts-on`** — owes about, examples, hostile, perf,
      tests. The roster of compiler-recognized names is at
      `docs/reference/lang/90-attributes.md:115`; `rule:attributes/attach-sites-and-forms` is why a
      userland shape that looks like one is not one.
- [ ] **`lang:attributes/core-program-implementing-i-every-class-implementing-an-interface`** — owes
      about, examples, hostile, perf, tests. The chapter's own worked example is at
      `docs/reference/lang/90-attributes.md:533`, and `crates/nvs-stdlib/src/program.rs:1` says why
      the call never runs. `tests/conformance/core/command-a-table-reads-back-every-command-and-option-a-program-declares.nvst`
      already drives `implementing<I>()` in one file, which is the idiom an example wants.

## Backlog

- `lang:attributes/core-route-and-core-access-the-route-table` and
  `…/core-json-derive-and-core-json-field-a-class-with-a-json-codec` are the group's last two, and
  both have landed conformance cases to build on (`tests/conformance/core/a-route-table-…`,
  `…/json-derive-encodes-declared-fields.nvst`) — `docs/agent/goals/` owns the order.
- **A folded `null` read through `?->` throws.** `open_nullsafe`
  (`crates/nvs-ir/src/lower/expr.rs:1118`) returns no guard unless the receiver lowered to
  `Ty::Tagged`, so `Core\Attributes::get<S>(C::m(...))?->field` reads a null. One slice, with a
  `.nvst` case pinning both spellings.
- `docs/reference/lang/90-attributes.md`'s `#[Core\Api]` paragraph and its JSON block were both
  stale about what the emitted document carries; the rest of the chapter was not re-read against the
  binary.
