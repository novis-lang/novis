# Handoff

## State

**Goal 11 stage 6 is all but whole: the lint, the renderer and the end-to-end example are on disk.**
Every `-p nvs-cli` test the goal's stage-6 check names passes, and the driver's failing check —
`examples/doc-comments.nvs` — now prints its four lines. One acceptance check is left in the whole
goal, and it is the next group below.

- **`nvs check --strict-docs` reports a public member with no `///`** (`E0326`, `rule:tooling/strict-docs`).
  It rides in the walk that already resolves a doc comment's tags rather than in a second one:
  `crates/nvs-hir/src/members.rs`'s `check_documented`, reached from `check_members`. The flag threads
  `nvs-cli`'s `run_check` → `front_end_granted` → `nvs_hir::resolve_program_linted` →
  `MemberResolver::check`, which is the shape `front_end_granted` already uses for the `[capabilities]`
  question: one entry point per caller that asks, so the plain `resolve_program`'s five call sites did
  not move.
- **What it reports is a *member*** — a method, a property, a class constant — and public is the absence
  of `private` and `protected`. A class, an interface, an enum, an enum case and a `type` alias are
  deliberately outside it; `crates/nvs-hir/src/members.rs`'s module doc is that boundary's home, and
  widening it is the publisher's call, not a session's.
- **`nvs doc <entry> [--out dir]` writes one Markdown page per class, interface and enum**, from
  `nvs meta --json`'s document and nothing else (`crates/nvs-cli/src/doc.rs`). A `@see` whose target has
  a page in the same run becomes a link into it; every other target stays code.
- **A program member now carries `visibility` in the JSON**, which the registry half has no counterpart
  for. `crates/nvs-cli/src/meta.rs` § *The program half* is that key's home. It is what lets a page show
  the surface a package's reader can reach, and it is additive — `tools/reference.py` and the website
  read the registry half, which is byte-identical still.
- Nothing is blocked. `python tools/verify.py` is green.

## Next group

**The last check in the goal: the doc-comment conformance suite** — one new directory,
`tests/conformance/syntax/doc-comment/`, which does not exist yet. `nvs test` runs a case with no
flags, so `E0326` cannot be pinned here; it is pinned by `crates/nvs-cli/tests/strict_docs.rs` instead.

- [ ] **A green case: a `///` run attaches to the declaration below it and changes nothing about the
      run** — `rule:tooling/doc-comment-attaches-to-the-next-declaration`. Prose, both tags, an ordinary
      `//` comment and a `////` run in one file, `--EXPECT--` the program's own output.
      `examples/doc-comments.nvs:1` is the same surface as a runnable program and is what to copy;
      `crates/nvs-hir/src/members.rs:513` is the walk that must stay silent about it.
- [ ] **Two `--EXPECTF-ERROR--` cases for the parser's half** — an unattached run (`E0127`) and an
      unknown `@tag` (`E0128`), `rule:tooling/doc-comment-tags-are-see-and-example`. The codes and their
      reasoning are `crates/nvs-diagnostics/src/lib.rs:208` and `crates/nvs-diagnostics/src/lib.rs:216`;
      an error case has to reproduce the diagnostic's own indentation, which widens with the line number.
- [ ] **Three `--EXPECTF-ERROR--` cases for the two tags that must resolve** — `E0323` at
      `crates/nvs-hir/src/members.rs:525`, and `E0324`/`E0325` at
      `crates/nvs-hir/src/members.rs:694`. The playbook bullet added this session says why an `@example`
      path inside a walked directory still has to name one.

## Backlog

- `nvs doc` writes a flat directory and no index page; widening that is
  `rule:tooling/nvs-doc-renders-and-decides-nothing`'s call, and it is kept cheap to replace on purpose.
- The goal's `[context] rules` names ADR numbers, which print one line per rule; a rule the item's work
  *implements* needs its fragment whole, so `rules = [...]` wants the `tooling/strict-docs`-style tokens
  beside the numbers. This session peeked three of them by hand.
- The goal's `[context] modules` still does not name `crates/nvs-hir/src/members.rs`, which the last two
  sessions have both edited.
- `docs/plan/m4b.md`'s hover row reads the `TriviaKind::DocComment` run this goal landed; nothing is owed
  there, per the goal's stage 6 § 3.
