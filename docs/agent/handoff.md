# Handoff

## State

**Goal 15 stage 3 is whole but for type annotations.** `editors/vscode/syntaxes/nvs.tmLanguage.json`
colours the openers and the shebang, comments and `#[...]` attributes, every string literal with
heredoc and nowdoc among them, the reserved table with the contextual spellings beside it, numeric and
duration literals, and now every name: `$var`, `$this`, the member after `->`, `?->` or `::`, the
`::class` constant that is not one, and the class, interface, enum, function and namespace names a
declaration introduces. A legacy cast `(int)$x` is consumed and coloured as nothing, so the type
keyword keeps its colour only where it keeps its meaning.

`npm run test:headless` is green at 90 and `npm run lint` is clean.

**The acceptance check `vscode (headless)` is still red on `protocol:` alone** — stage 5's suite,
which cannot go green before then. `grammar:` and `contributions:` both print.

**Type annotations are the one construct family of `rule:ide/highlighting-is-two-layers` the grammar
still does not colour.** `function f(int $x): decimal` colours both keywords from the reserved table
and nothing because they stand in a type position; `type Id = int` colours `type` and leaves `Id`
plain; the inline shape `{x: int}` is untouched.

**Still no `src/` and no `main`** — stage 5's, and stage 8's `.vsix` wants `main` before it is worth
running. The pack's `[context] modules` warning about `editors/vscode/src/**` is that absence, not a
manifest bug.

## Next group

**Stage 3's last family: type annotations** — one file set: the `#code` include list at
`editors/vscode/syntaxes/nvs.tmLanguage.json:48`, the `#names` entry at `:87` whose declaration
patterns are the shape to copy, the allowlist at `editors/vscode/test/grammar/allowlist.test.ts:37`,
and a fixture plus a `*.test.ts` beside them. It is one group and not three because a type name, a
type slot and the alias's own name are one pattern family sharing one new allowlist row.

- [ ] **Type annotations in every slot** — parameter, return, property, class constant, `foreach`
      binding, typed local, and the inline shape `{x: int}`, added at
      `editors/vscode/syntaxes/nvs.tmLanguage.json:48`. `rule:ide/highlighting-is-two-layers` lists
      the slots and `rule:types/object-top` owns the inline shape. The scalar words already colour
      from the reserved table at `:145`, so the slice is the positions and the *named* types in them,
      which want a new `entity.name.type.nvs` row.
- [ ] **The name a `type` alias introduces** — `type Id = int` leaves `Id` plain because the
      contextual rule at `editors/vscode/syntaxes/nvs.tmLanguage.json:203` matches by lookahead and
      consumes no name. The declaration patterns at `:87` are the shape that fixes it.
- [ ] **A fixture line per new scope** — `editors/vscode/test/grammar/allowlist.test.ts:127` fails on
      an allowlist row no fixture reaches, so a scope lands with the line that produces it.

## Backlog

- `editors/vscode/src/` and a `main`: stage 5's client and stage 8's `.vsix` — `docs/agent/loop-goal.md`.
- A static property `Foo::$bar` leaves the `::` without its accessor scope — `#names`, `nvs.tmLanguage.json:87`.
- A bare call `checkout($x)` colours no name; only a member and a declaration do — same entry.
- `with` after a `spawn script` path stays uncoloured — the `contextual-keywords` comment owns it.
- The second grammar, for `.nvst` and `.lspt` themselves — ADR 0099 § 4.
