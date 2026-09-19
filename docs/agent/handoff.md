# Handoff

## State

Goal `lang:programs`, two of its nine features finished with all four proofs plus `about.md`:
`a-complete-program-annotated` and `a-program-is-a-file-of-top-level-statements`. Both measured into
`docs/perf/members.ndjson`; `python tools/dossier.py --verify --group lang:programs` now names seven
features owing, and the two above are not among them.

The roster had a tenth, phantom feature: `chapter_features` in `tools/dossier.py` read every `# `
line in a reference chapter as a heading, including the `# also a line comment` inside the sample in
`docs/reference/lang/10-programs.md`. It now skips fenced blocks whole. That was the group's third
listed item, so there is no proof work behind it — `docs/agent/loop-goal.md` still lists it and the
next `python tools/dossier.py --emit-goals` clears that line.

## Next group

Three more features of the same chapter — one file set: `docs/reference/lang/10-programs.md`, plus a
fresh directory each under `docs/examples/lang/programs/`, `tests/hostile/lang/programs/` and
`benches/members/lang/programs/`. Each item is one feature with `about.md` and all four proofs, in
the shapes `docs/examples/README.md`, `tests/hostile/README.md` and `benches/members/README.md` own
(`rule:testing/four-proofs`).

- [ ] **`lang:programs/comments`** — owes about, 3 examples, 2 tests, hostile, perf.
      `docs/reference/lang/10-programs.md:109`
- [ ] **`lang:programs/code-mode-and-html-mode`** — owes about, 3 examples, 2 tests, hostile, perf.
      The `.out` of an example here holds the HTML-mode bytes verbatim.
      `docs/reference/lang/10-programs.md:57`
- [ ] **`lang:programs/names-and-casing`** — owes about, 3 examples, 2 tests, hostile, perf.
      `docs/reference/lang/10-programs.md:125`

## Backlog

- `Core\Json::encode` refuses a nest of 1024 empty arrays while the message says "past 1024 levels":
  `DEPTH_CEILING_FRAMES` is `DEPTH_CEILING - 1` (`crates/nvs-stdlib/src/json.rs:936`, checked at
  `:1436`), which is right when the innermost value is a scalar and one strict when it is `[]`. Only
  the degenerate case differs; nothing crashes.
- A chapter feature's anchor is one line past its `#` heading (`tools/dossier.py:663` onwards counts
  the line before testing it). Harmless — `orient.py` windows around it — but every `implemented at`
  is off by one.
- `docs/agent/loop-goal.md` lists `lang:programs/also-a-line-comment`, which no longer exists;
  `--emit-goals` is what rewrites it, and nothing should hand-edit the generated file.
- The remaining four of the group after the next: `namespaces-and-use`,
  `require-run-another-file-in-this-frame`, `autoload-find-a-class-by-its-namespace`,
  `ending-a-program`.
