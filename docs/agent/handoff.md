# Handoff

## State

**Goal 15 stage 3 is nearly whole.** `editors/vscode/syntaxes/nvs.tmLanguage.json` now colours the
openers and the shebang, comments and `#[...]` attributes, every string literal with heredoc and nowdoc
among them, the whole reserved table with the contextual spellings beside it, and numeric and duration
literals. `npm run test:headless` is green at 76 and `npm run lint` is clean.

**The acceptance check `vscode (headless)` is still red on `protocol:` alone** — stage 5's suite, which
cannot go green before then. `grammar:` and `contributions:` both print.

**Code mode still colours no names.** No `$var`, no `$this`, no class or function name: the keyword
patterns guard with `(?<![$\w])(?<!->)(?<!::)` rather than relying on a variable rule to consume `$if`
first, so the next group may add names without unpicking them. `with` after a `spawn script` path is
uncoloured on purpose — no anchoring token is close enough for a regex, and the grammar's own
`contextual-keywords` comment is the home of that.

**Still no `src/` and no `main`** — stage 5's, and stage 8's `.vsix` wants `main` before it is worth
running. The pack's `[context] modules` warning about `editors/vscode/src/**` is that absence, not a
manifest bug.

## Next group

**Stage 3, the last of the grammar** — one file set: the `#code` entry at
`editors/vscode/syntaxes/nvs.tmLanguage.json:48`, the allowlist at
`editors/vscode/test/grammar/allowlist.test.ts:37`, and a fixture plus a `*.test.ts` beside them. Names
go first: they split spans the other suites assert on, and every suite written after them is written
once.

- [ ] **Names in code mode** — `$var` and `$this`, the member after `->` or `::`, and the class,
      function and namespace names a declaration introduces, added at
      `editors/vscode/syntaxes/nvs.tmLanguage.json:48`. `rule:ide/novis-ships-names-not-colours` is the
      vocabulary; `variable.other.nvs` and `variable.language.nvs` are already on the allowlist from the
      string slice, `entity.name.*` is not.
- [ ] **Type annotations in every slot**, the inline shape `{x: int}` among them, at
      `editors/vscode/syntaxes/nvs.tmLanguage.json:48`. What a type position takes is
      `crates/nvs-syntax/src/parser/ty.rs:306`, and `rule:types/object-top` is the shape's rule; the
      scalar keywords already colour, so what is left is the positions a name reaches one.
- [ ] **What must not colour as though it were valid** — `===`/`!==`, `(int)$x`, `|>` and
      `if (...): ... endif;` — as negative cases in a new
      `editors/vscode/test/grammar/rejected.test.ts` beside
      `editors/vscode/test/grammar/openers.test.ts:1`. `rule:ide/rejected-syntax-gets-no-colour`. The
      alternative colon syntax needs no pattern at all: `endif` is in no reserved table, so nothing
      colours it today and the case pins that.

## Backlog

- The `.nvst`/`.lspt` grammar, whose bodies include this one — ADR 0099 § 4.
- Stage 5's client: `src/`, `main`, and the protocol suite the acceptance check still waits on —
  `docs/agent/loop-goal.md`.
- The extension-host tier stays CI's and is never run here — `docs/agent/loop-goal.md` § *Standing
  decisions*.
