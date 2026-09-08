# Handoff

## State

**Goal 15 stage 3 is most of the way in.** `editors/vscode/syntaxes/nvs.tmLanguage.json` exists and is
contributed at `editors/vscode/package.json:26`: the dual-mode openers, the shebang, comments and
`#[...]` attributes. A new `editors/vscode/test/grammar/` tokenizes fixtures with `vscode-textmate` and
`vscode-oniguruma` — two new devDependencies, so the lockfile moved — and freezes 24 cases. `npm run
lint` and `npm run test:headless` are green at 39.

**The acceptance check `vscode (headless)` is now red on `protocol:` alone.** `grammar:` prints, so the
one `want` line still missing is stage 5's protocol suite; it cannot go green before then.

**The `#code` entry at `editors/vscode/syntaxes/nvs.tmLanguage.json:48` is where every remaining family
lands**, and the order matters: a keyword or type rule cannot know it is inside a string or a comment,
so whatever consumes those has to be offered the position first.

**Still no `src/` and no `main`** — stage 5's, and stage 8's `.vsix` wants `main` before it is worth
running.

## Next group

**Stage 3, the rest of the grammar** — one file set: the `#code` entry at
`editors/vscode/syntaxes/nvs.tmLanguage.json:48`, the allowlist at
`editors/vscode/test/grammar/allowlist.test.ts:37`, and a fixture plus a `*.test.ts` beside them. Take
them in this order; the first is a prerequisite of the second, not a preference.

- [ ] **Strings, heredoc and nowdoc** — interpolation inside `"` and inside a heredoc, none inside `'`
      or a nowdoc, added at `editors/vscode/syntaxes/nvs.tmLanguage.json:48`.
      `rule:ide/highlighting-is-two-layers`; the lexer's heredoc opener is
      `crates/nvs-syntax/src/lexer.rs:1113`.
- [ ] **Keywords, types and qualifiers** at `editors/vscode/syntaxes/nvs.tmLanguage.json:48`. The whole
      reserved table is `crates/nvs-syntax/src/token.rs:424`, matched exactly and in lower case only, so
      `IF` is an identifier and `tainted`, `secret` and `decimal` are reserved words rather than
      contextual ones. What a type position takes is `crates/nvs-syntax/src/parser/ty.rs:318`, and
      `type`, `by`, `get`, `set`, `from` and `spawn` lex as plain identifiers recognised by text one
      position at a time (`crates/nvs-syntax/src/parser/mod.rs:609`).
      `rule:ide/highlighting-is-two-layers`.
- [ ] **Type annotations in every slot**, the inline shape `{x: int}` among them, and duration
      literals, at `editors/vscode/syntaxes/nvs.tmLanguage.json:48`. `rule:types/object-top`,
      `rule:types/duration-literal`.
- [ ] **What must not colour as though it were valid** — `===`/`!==`, `(int)$x`, `|>` and
      `if (...): ... endif;` — as negative cases in a new
      `editors/vscode/test/grammar/rejected.test.ts` beside
      `editors/vscode/test/grammar/openers.test.ts:1`. `rule:ide/rejected-syntax-gets-no-colour`.

## Backlog

- Stage 4's grammar for `.nvst` and `.lspt` is the same directory and the same harness, and its bodies
  include this grammar: `rule:ide/case-files-have-their-own-grammar`.
- Stage 5 (the client, and the `main` the manifest still lacks) and stage 7 (Tasks, the problem matcher,
  the AST panel) share `src/`. Stage 6 (`secret` concealment) needs the client.
- Stage 8's extension-host suite is **not on the acceptance list and must not be added** — CI owns it.
  `.vsix` packaging does gate, at stage 8, and wants `main` first.
- The openers are an injection, and it is proved against a one-rule `text.html.basic` stub in
  `editors/vscode/test/grammar/tokenize.ts:31`. The real HTML grammar is an editor's rather than an npm
  package, so nesting against all of it can only be checked in the host tier.
- Stage 3's `[context] rules` gained `statements/nvs-is-the-only-open-tag`,
  `classes/reserved-spellings-are-lower-case` and `tooling/doc-comment-is-three-slashes`: the grammar
  colours what the lexer accepts, and this session had to fetch all three.
- **When this goal's last check goes green M4B is finished**, and the driver takes goal 16 — the body
  rule and `Core\Request::json()`/`jsonAs<T>()` — then goal 17, `Core\Test::request`'s shape.
