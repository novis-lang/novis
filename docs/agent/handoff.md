# Handoff

## State

Goal `lang:programs`, six of its nine features finished with all four proofs plus `about.md`:
the four that were already done, and now `names-and-casing` and `namespaces-and-use`. `python
tools/dossier.py --verify --group lang:programs` names three owing, all of them in the handoff's
next group. Both new benches declare `// bench: allocations 0` and both measure it; the figures
are in `docs/perf/members.ndjson`.

One finding came out of the namespace proofs and is in `## Backlog` rather than in code: a second
`namespace` declaration in one file parses and takes effect, so every declaration after it
resolves under the newer name, which `docs/reference/lang/10-programs.md` § *Namespaces and
`use`* does not allow. No rule fragment decides it either way and no milestone at M9 or later
scopes it, so `python tools/owners.py --check` refuses the `# Known gaps` item it would otherwise
be — whether to refuse a second declaration is a call for the user.

Two of the three remaining features — `require` and `autoload` — each need a second file beside
the example program, and whether an example directory may hold one is `docs/examples/README.md`'s
to say. Not checked this session.

## Next group

**Stage 2: the dossier, one feature per slice** — one file set: `docs/reference/lang/10-programs.md`,
a fresh directory each under `docs/examples/lang/programs/`, `tests/hostile/lang/programs/` and
`benches/members/lang/programs/`, and two `.nvst` cases under `tests/conformance/`. Each item is
one feature with `about.md` and all four proofs, in the shapes `docs/examples/README.md`,
`tests/hostile/README.md` and `benches/members/README.md` own (`rule:testing/four-proofs`).

- [ ] **`lang:programs/require-run-another-file-in-this-frame`** — owes about, 3 examples, 2 tests,
      hostile, perf. `docs/reference/lang/10-programs.md:201`
- [ ] **`lang:programs/autoload-find-a-class-by-its-namespace`** — owes about, 3 examples, 2 tests,
      hostile, perf. `docs/reference/lang/10-programs.md:229`
- [ ] **`lang:programs/ending-a-program`** — owes about, 3 examples, 2 tests, hostile, perf.
      `docs/reference/lang/10-programs.md:250`

## Backlog

- A second `namespace` declaration per file is accepted and re-points every declaration after it;
  the chapter allows one. Verified with `target/debug/nvs.exe check`. Needs a user call, since
  refusing it is a new `E01xx` code and a rule fragment, and nothing owns it today.
- `rule:core-api/casing-checks-the-leading-character` says only the first character's case is
  checked, and a class constant is checked whole: `public const int MaxLines = 1;` is `E0113` with
  the rename `MAX_LINES`. The fragment owes a clause naming SCREAMING_SNAKE_CASE as the exception.
- `docs/reference/lang/10-programs.md` § *Names and casing* says "No identifier may start with
  `_`" with no diagnostic of its own; `_hidden` is reported as the camelCase error whose rename
  drops the underscore. Accurate, and worth a sentence in the chapter saying so.
