# Handoff

## State

Goal `lang-concurrency` is **reached**. `python tools/dossier.py --verify --group lang:concurrency`
reports nothing owed across all ten features, `examples: 30 ok, 0 failed` and `hostile: 10 ok,
0 failed`. The three gates a goal meets only at its end are green too: `verify.py --doc` resolves
every link, and `owners.py --closes lang-concurrency` and `playbook.py --closes lang-concurrency`
each report the goal owns no gap.

The tree that would not compile last session does now. The uncommitted `crates/nvs-lsp/` edit that
made `completion::at` take six arguments against a five-argument declaration was the person at the
keyboard's, and they landed it themselves as `fa358ee3e` and `985a38bfd`; the working tree is clean
and `python tools/verify.py` is 14 of 14 green (4903 tests, 2221 conformance, 279 differential).

Landed this session: the playbook bullet about a release build an editor's `nvs lsp` blocks now says
`dossier.py` and `loop.py` retry it through `tools/relink.py`, so only a by-hand build still needs
the server stopped.

## Next group

**Goal `lang-attributes`, stage 1: one slice is one feature, all its feature proofs together** — one
file set: `docs/reference/lang/90-attributes.md` and each feature's own four proof trees. Every item
owes examples, hostile, perf and tests plus its `about.md`, per `rule:testing/feature-proofs`. The
goal file's § *Running this goal wide* is the fan-out protocol; `python tools/dossier.py --partition
--group 'lang:attributes'` is what writes the briefs.

- [ ] **`lang:attributes/an-attribute-is-a-shape-literal-attached-to-a-declaration`** — the base
      syntax every other item builds on, so it goes first.
      `docs/reference/lang/90-attributes.md:9`
- [ ] **`lang:attributes/reading-attributes-back-core-attributes-get-and-all`** — the read side of
      the same section, sharing its examples' vocabulary.
      `docs/reference/lang/90-attributes.md:93`
- [ ] **`lang:attributes/the-names-the-compiler-acts-on`** — the roster the two above are read
      against, adjacent in the same chapter.
      `docs/reference/lang/90-attributes.md:116`

## Backlog

- The five remaining `lang:attributes` features — `core-json-derive`, `core-route`, `core-api`,
  `core-command`, `core-program-implementing-i` — are in `docs/agent/goals/dossier/88-lang-attributes.md`.
- Goal `the-description-is-owed` is where the `about.md` check is switched on; until then only the
  goal file's own paragraph asks for one.
