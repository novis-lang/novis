# Handoff

## State

**Goal 15 stage 3 is whole.** `editors/vscode/syntaxes/nvs.tmLanguage.json` colours every construct
family [ADR 0099 § 4](../decisions/0099.md) names: the openers and the shebang, comments and `#[...]`
attributes, every string literal, the reserved table with the contextual spellings beside it, numeric
and duration literals, every name — and now a written type in every slot
`rule:types/declaration` gives one, the name a `type` alias introduces, and
`rule:types/object-top`'s inline shape. `npm run test:headless` is green at 99 and `npm run lint` is
clean.

**The acceptance check `vscode (headless)` is still red on `protocol:` alone** — stage 5's suite,
which cannot go green before then. `grammar:` and `contributions:` both print.

**Two slots are deliberately narrower than `rule:types/grammar` allows**, and both are pinned in
`editors/vscode/test/grammar/types.test.ts`: a return type is read only inside a `#signature` region,
because a ternary's colon and a return type's are one spelling; an inline shape only inside a type
region, because the object literal `{n: $n * 10}` is written the same way as `{n: int}`.

**Still no `src/` and no `main`** — stage 5's, and stage 8's `.vsix` wants `main` before it is worth
running. The pack's `[context] modules` warning about `editors/vscode/src/**` is that absence, not a
manifest bug.

## Next group

**Stage 4: the `.nvst`/`.lspt` grammar** — one file set: `editors/vscode/syntaxes/`,
`editors/vscode/package.json` and the two suites under `editors/vscode/test/` that freeze the manifest.
The goal calls it nearly free and says to group it with stage 3: same directory, same harness, and it
is the cheapest check that stage 3's grammar is embeddable at all.

- [ ] **A second grammar for the case formats** — `editors/vscode/syntaxes/nvst.tmLanguage.json`, a
      `begin`/`end` per section anchored on `^--NAME--$` and ending at `(?=^--[A-Z])`, with
      `source.nvs` included in the four sections that hold a program: `--FILE--`, `--FILE <path>--`
      (its own rule, since that header carries an argument), `--SKIPIF--` and `--CLEAN--`. The section
      list is `crates/nvs-test/src/lib.rs:1`'s module doc — read it rather than inferring the set from
      the corpus, and every section it names that this stage does not is literal text with only its
      delimiter coloured. `rule:ide/case-files-have-their-own-grammar`.
- [ ] **`--ORACLE--` is PHP and `--EXPECTF--` is escapes** — `constant.character.escape` on the `%`
      escapes in `--EXPECTF--` and `--EXPECTF-ERROR--`, and the PHP leg settled in the snapshot test
      rather than from documentation: VS Code splits PHP across `source.php` and `text.html.php` and
      only the latter opens on the `<?php` every oracle body starts with. The registry that decides
      what a scope name loads is `editors/vscode/test/grammar/tokenize.ts:56`, and the HTML stub above
      it at `:32` is the shape to copy for a leg the headless registry has no package for.
- [ ] **The manifest gains a second language and a second grammar** — `editors/vscode/package.json`,
      against the two assertions that freeze it today: `editors/vscode/test/grammar/allowlist.test.ts:87`
      deep-equals `contributes.grammars` to the one `nvs` entry, and
      `editors/vscode/test/contributions/contributions.test.ts:99` holds `contributes.languages` at
      one. Read `contributions.test.ts:111` — "names php nowhere in the manifest" — before embedding
      PHP anywhere, since a `.nvst` grammar naming `text.html.php` is not the `.php` claim that test
      exists to refuse (`rule:ide/the-extension-claims-nvs-only`).

## Backlog

- `callable(T): U` in a *parameter* leaves `T` uncoloured — that argument list is not a type region.
- A shape type spanning two lines closes its region at the opening `{`; one line is what colours.
- No `editors/vscode/src/` and no `main` yet — stage 5's, and stage 8's `.vsix` waits on it.
- The extension-host tier stays off this machine on purpose — goal § *Standing decisions*.
