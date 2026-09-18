# Handoff

## State

**Goal `one-type-test`: `crates/` is clean of the refused word.** The gate's grep over `crates` now
returns nothing outside its four excluded files, so what stage 7 still owes is prose under `docs/` and
`tests/`, plus the status flip.

**What is left for the gate**, by file set: the eight rule fragments under `docs/rules/` listed in
§ *Next group* (their chapters are `python tools/rules.py --render`'s output and are never hand-edited —
`tools/rules.py`'s module doc); `docs/reference/findings.md:76`, `:96` and
`docs/reference/tools/30-php-differences.md:5`, `:60`, `:119`, `:164`; `docs/spec/01-core-library.md:75`
and `docs/spec/02-php-migration.md:934`, `:941`, `:942`, `:944`;
`tests/conformance/reject/new-takes-a-class-name-not-a-string.nvst:2`, `:9`; and ten bullets in
`docs/agent/playbook.md`, several of which are traps whose whole subject is the removed operator and are
`[until:]`-closable rather than rewordable.

**Two files state the divergence and only one is excluded from the gate.**
`docs/rules/php-migration/every-divergence-is-deliberate-and-listed.md:21` and
`docs/rules/types/type-test.md:92` both name the word to point at
`rule:php-migration/one-type-test`, and neither is in the check's exclusion list
(`docs/agent/loop-goal.toml:11615`). One of the two answers has to be taken for both: add the pair of
paths to the exclusion list, or reword each to cite the rule without spelling the word. The second is
the safer one — it keeps the gate's exclusion list exactly the refusal plus the rule that owns the
divergence — and it is what § *Next group*'s last item assumes.

**`docs/rules/php-migration.json:199` still reads `"status": "designed"`** where the gate wants
`shipped`, and the hole behind it is unchanged: `$m is Core\Str` type-checks and dies at codegen,
`crates/nvs-codegen/src/emit.rs:2678`, where `rule:types/type-test` says a knowable answer folds to
`false`. The comment at `:2673` names `expr::members::testable_class_name`, which does not exist; the
function is `testable_core_class`.

## Next group

**Stage 7: the rule fragments, then one render** — one file set: `docs/rules/` plus
`python tools/rules.py --render`, which rewrites the chapters the fragments feed and is the only way
`docs/rules/classes.md`, `core-api.md`, `expressions.md` and `types.md` lose their hits. None of these
lines is a diagnostic string or a `.nvst` assertion, so nothing in the suite can go red on the wording;
`rule:types/type-test` is the same test under its living name, and the two that *compare* the spellings
cite `rule:php-migration/one-type-test` instead of naming the word (§ *Standing decisions*).

- [ ] **The four class-and-interface fragments** — `docs/rules/classes/no-traits.md:8`,
      `docs/rules/classes/clone-is-shallow.md:6`,
      `docs/rules/classes/interface-default-methods.md:2`,
      `docs/rules/core-api/one-paradigm-per-operation.md:12`. Each names the operator only to say what
      a value is or is not testable as; `$x is C` is that sentence under `rule:types/type-test`.
- [ ] **The four type-and-expression fragments** — `docs/rules/types/callable-absorbs-closure.md:9`,
      `docs/rules/types/class-constant.md:21`, `docs/rules/types/class-reference-sites.md:22`,
      `docs/rules/expressions/catch-lowers-to-block-form.md:3`. `class-reference-sites.md`'s line says
      PHP spells the site that way, which `rule:php-migration/one-type-test` now owns outright.
- [ ] **The two divergence citations, then the render** —
      `docs/rules/php-migration/every-divergence-is-deliberate-and-listed.md:21` and
      `docs/rules/types/type-test.md:92`, reworded to cite `rule:php-migration/one-type-test` without
      spelling the word, then `python tools/rules.py --render` for every chapter this group touched.
      § *State* has why rewording beats widening the gate's exclusion list.

## Backlog

- `docs/rules/php-migration.json:199` is `designed`, and one of stage 7's four checks wants `shipped`.
- `crates/nvs-codegen/src/emit.rs:2678` — `$m is Core\Str` type-checks and dies at codegen.
- `docs/reference/` and `docs/spec/` hits, one file set of their own — § *State* lists the anchors.
- Ten `docs/agent/playbook.md` bullets; `tools/playbook.py`'s `[until:]` closes some rather than editing.
- `tests/conformance/reject/new-takes-a-class-name-not-a-string.nvst:2`, `:9` — case prose, not an
  assertion.
