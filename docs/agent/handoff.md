# Handoff

## State

Goal `lang:programs`, four of its nine features finished with all four proofs plus `about.md`:
`a-complete-program-annotated`, `a-program-is-a-file-of-top-level-statements`, `comments` and
`code-mode-and-html-mode`. `python tools/dossier.py --verify --group lang:programs` now names five
features owing, and none of those four is among them.

Two findings came out of the proofs and both are fixed. The chapter's bullet on `#!` said a shebang
line "is not recognized in this build — it is HTML-mode text", which contradicts
`rule:tooling/shebang-opens-code-mode` and the binary; it is rewritten, and the new reject case pins
both halves. That case then failed the lossless corpus: the `E0009` recovery consumed the refused
`<?nvs` without recording trivia for it, so `rule:ide/tokens-plus-trivia-reproduce-the-file` did not
hold over any file holding one (`crates/nvs-syntax/src/lexer.rs:561`). The chapter edit restaled
every perf figure in it; all are re-measured and current.

## Next group

Three more features of the same chapter — one file set: `docs/reference/lang/10-programs.md`, plus a
fresh directory each under `docs/examples/lang/programs/`, `tests/hostile/lang/programs/` and
`benches/members/lang/programs/`. Each item is one feature with `about.md` and all four proofs, in
the shapes `docs/examples/README.md`, `tests/hostile/README.md` and `benches/members/README.md` own
(`rule:testing/four-proofs`).

- [ ] **`lang:programs/names-and-casing`** — owes about, 3 examples, 2 tests, hostile, perf.
      `docs/reference/lang/10-programs.md:127`
- [ ] **`lang:programs/namespaces-and-use`** — owes about, 3 examples, 2 tests, hostile, perf.
      `docs/reference/lang/10-programs.md:153`
- [ ] **`lang:programs/require-run-another-file-in-this-frame`** — owes about, 3 examples, 2 tests,
      hostile, perf. `docs/reference/lang/10-programs.md:201`

## Backlog

- `?>` inside a `//` or `#` comment does not leave code mode — the line comment runs to the newline
  and swallows it (`crates/nvs-syntax/src/lexer.rs:471`). PHP's closing tag wins inside a line
  comment, so this is a divergence that `rule:php-migration/every-divergence-is-deliberate-and-listed`
  does not list. It needs a decision — reproduce PHP, or list it — and no proof written here asserts
  either way.
- `lang:programs` still owes `autoload-find-a-class-by-its-namespace` and `ending-a-program` after
  the group above; `docs/reference/lang/10-programs.md:229` and `:250`.
- `nvs fmt` indents the `}` of a `try`/`catch` whose body leaves code mode one level too far
  (`tests/hostile/lang/programs/code-mode-and-html-mode/01-…:51-54`). It is a fixed point, so the
  corpus is stable and nothing is red; it is a layout bug for whoever owns `crates/nvs-fmt/src/indent.rs`.
