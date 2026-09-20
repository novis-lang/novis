# Handoff

## State

Goal `lang-attributes`, 4 of the group's **8** features complete in `python tools/dossier.py --gate
--group lang:attributes`: `an-attribute-is-a-shape-literal-attached-to-a-declaration`,
`core-api-and-the-openapi-document`, `core-command-and-core-option-the-command-table` and
`reading-attributes-back-core-attributes-get-and-all`. Nothing is blocked.

The retrieval slice found and fixed a real bug: a `$member` argument that is not a string literal was
ignored, so the retrieval answered the *target's own* roster instead of the empty result
`rule:attributes/structural-retrieval` fixes for that case. The fix is
`crates/nvs-types/src/retrieval.rs:347`, and the module doc there already stated the intended
behaviour, so nothing in `docs/` moved.

Two things the landed features settled, so the next session does not re-derive them. **A `lang:`
feature whose perf figure is measured goes stale when `docs/reference/lang/90-attributes.md` is
edited** — that chapter is what every `lang:attributes/…` feature is "implemented at" — so a slice
that corrects the chapter re-measures with `--record-perf --only` before it wraps. And **`#[Core\Api]`
has no runtime surface at all**: `Core\Attributes::get` answers `null` for it, so its perf proof is a
`[skip]` entry in `tools/data/dossier-policy.toml` with the reason, while `#[Core\Command]` and
`#[Core\Option]` *are* readable and carry a real bench.

## Next group

**Stage 2: the dossier, the compiler-read attribute names over one reference chapter** — one file
set: `docs/reference/lang/90-attributes.md`, `docs/examples/lang/attributes/`,
`tests/hostile/lang/attributes/`, `benches/members/lang/attributes/`, `tests/conformance/lang/`.
All three read the same chapter and all three are about attributes the *compiler* acts on, so the
proofs share their idiom with the four that landed.

- [ ] **`lang:attributes/the-names-the-compiler-acts-on`** — owes about, examples, hostile, perf,
      tests. `rule:attributes/inert-metadata` is the line between an inert attribute and one the
      compiler reads; the closed set and the `<!-- generated: attributes -->` table are at
      `docs/reference/lang/90-attributes.md:115`, and the roster the table is generated from is
      `crates/nvs-types/src/attributes.rs:1`.
- [ ] **`lang:attributes/core-route-and-core-access-the-route-table`** — owes about, examples,
      hostile, perf, tests. `rule:attributes/access-is-a-required-sibling` and
      `rule:attributes/access-payload` are what the pair owes; the option roster for both is
      `docs/reference/lang/90-attributes.md:244`.
- [ ] **`lang:attributes/core-program-implementing-i-every-class-implementing-an-interface`** —
      owes about, examples, hostile, perf, tests. `rule:programs/implementing` is the rule; the
      member's reference card is `docs/reference/lang/90-attributes.md:533`.

## Backlog

- A folded `null` read straight through `?->` throws instead of short-circuiting; anchor
  `crates/nvs-ir/src/lower/expr.rs:1118`, workaround in `docs/agent/playbook.md`. No milestone at M9
  or later owns it, so `python tools/owners.py --check` refuses it as a recorded gap.
- `lang:attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec` is the fourth
  feature this group still owes; `docs/reference/lang/90-attributes.md:129`.
- An explicitly written `Core\Attributes::get<T>($target, "")` is `E0798` while omitting the
  argument is fine, because the default is `""`. No rule states which is right.
