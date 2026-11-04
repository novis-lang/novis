---
looper: 1
title: a .nvs file in VS Code reads as calmly as the same code in PhpStorm
summary: "the Novis themes draw brackets in the text colour, methods in PhpStorm's blue, variables in its muted purple and parameter hints without a box, and a setting turns the parameter-name hints off"
context: [editors/vscode/themes/novis-dark-color-theme.json, editors/vscode/themes/novis-light-color-theme.json, editors/vscode/src/colours.ts, editors/vscode/test/contributions/contributions.test.ts]
stages:
  - id: 1
    title: "the Novis themes draw brackets plainly, methods and variables in PhpStorm's colours, and hints without a box"
    status: open
    context: [editors/vscode/src/theme.ts, editors/vscode/test/client/colours.test.ts, docs/rules/ide/novis-ships-names-not-colours.md, docs/reference/tools/40-editor.md]
  - id: 2
    title: "nvs.inlayHints.parameterNames turns the parameter-name hints off, and the server notices without a restart"
    status: open
    context: [crates/nvs-lsp/src/settings.rs, crates/nvs-lsp/src/server.rs, crates/nvs-lsp/src/hints.rs, crates/nvs-lsp/tests/handshake.rs, editors/vscode/package.json, editors/vscode/test/client/settings.test.ts, docs/rules/ide/contributions-are-frozen-and-only-ever-added.md, docs/rules/ide/the-request-set-is-closed.md, website/src/content/docs/tools/editor/index.mdx]
  - id: 3
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: themes-calm
    stage: 1
    name: both Novis themes draw brackets in the text colour and hints without a box, and the setting removes a rule an earlier palette wrote
    run: [npm, --prefix, editors/vscode, run, test:headless]
    timeout: 900
    tests:
      - draws every bracket level in the theme's text colour, and a parameter hint without a box
      - removes a rule the setting wrote with an earlier palette
  - id: hints-setting
    stage: 2
    name: the server reads nvs.inlayHints.parameterNames, drops only the name hints, and asks the client to fetch hints again when it changes
    run: [cargo, test, --test, handshake, --, parameter_name_hints]
    timeout: 1800
    tests:
      - parameter_name_hints_turned_off_leave_only_the_type_hints
      - parameter_name_hints_turned_off_ask_the_client_to_fetch_hints_again
---

# a .nvs file in VS Code reads as calmly as the same code in PhpStorm

## Why

The user compared a `.nvs` file in VS Code under Novis Dark with a PHP file in PhpStorm. The VS Code
view is "too busy" and harder to follow. Screenshots of the two were compared pixel by pixel. Both
use the same font at the same size (about 7.6 px per character). The line pitch is 19.5 px against
20.3 px, which is too small a difference to explain it. The differences that do explain it:

1. **Rainbow brackets.** VS Code colours bracket pairs by depth by default (gold, orchid, blue). Novis
   Dark sets no `editorBracketHighlight.*` colour, so VS Code's default rainbow shows. PhpStorm draws
   every bracket in the text colour. The gold `(` sits next to a gold method name, so each call reads
   as one yellow clump.
2. **The method colour.** Novis Dark takes most of its palette from PhpStorm's current Dark theme
   (`#BCBEC4` text, `#CF8E6D` keywords, `#6AAB73` strings, `#5F826B` doc comments). Its method colour
   `#FFC66D` comes from the older Darcula theme and is the brightest colour on screen, often three
   times per line. Sampled from the PhpStorm screenshot: a function call is `#56A8F5`.
3. **The variable colour.** Novis Dark draws variables and parameters `#8BA7CF`, a light blue. Next to
   a blue method that is too close. Sampled from the PhpStorm screenshot: a variable is `#9876AA`, a
   darker, muted purple, and a static property is the brighter `#C77DBB`, which Novis Dark already
   uses for properties.
4. **Parameter hints in boxes.** Every literal argument gets a `name:` hint with VS Code's default
   grey box. The server already shows hints only before a literal, which is the same set PhpStorm
   shows. The extension has no setting to turn them off. Only VS Code's own `editor.inlayHints.enabled`
   does, and only for all hints at once.

Not this goal's: the VS Code machine draws `==`, `!=` and `->` as ligatures (`editor.fontLigatures`),
and PhpStorm does not. That is the user's own editor setting. The example file also writes
`Core\Time\DateTime` in full on every line where a `use` import would shorten it. That is the code,
not the extension.

## Standing decisions

**Colour stays the user's choice.** `rule:ide/novis-ships-names-not-colours` holds: every colour this
goal changes is in the two bundled themes, which apply only when the user picks one, or in the
`nvs.highlighting.distinctNames` palette, which applies only when the user turns it on. Nothing goes
into `configurationDefaults`.

**Novis Dark's new colours.**

- Methods and functions: `#56A8F5`. This covers the semantic `method` and `function` token types,
  and the TextMate fallback `entity.name.function`, `support.function` and `meta.function-call.generic`.
  `PALETTE.dark.method` in `editors/vscode/src/colours.ts:16` changes with it, because the contributions
  suite asserts the theme agrees with the palette.
- Variables and parameters: `#9876AA`. This covers the semantic `variable` and `parameter` token types
  and the TextMate `variable` and `variable.parameter` scopes. `variable.language` (`$this`, `self`)
  stays the keyword colour.
- Every bracket level, `editorBracketHighlight.foreground1` to `foreground6`, is the theme's
  `editor.foreground`. `editorBracketHighlight.unexpectedBracket.foreground` is the theme's `invalid`
  colour (`#F75464`). The matching-bracket box (`editorBracketMatch.*`) stays as it is.
- Inlay hints: `editorInlayHint.background`, `editorInlayHint.parameterBackground` and
  `editorInlayHint.typeBackground` are `#00000000`. The three matching foregrounds are the theme's
  comment colour (`#7A7E85`), so a hint reads as dim text, not as a box.

**Novis Light.** The light palette stays: `#795E26` methods and `#36598A` variables are not the
loud colours on a white background. The brackets and the inlay hints follow the same rule as in Novis
Dark: text colour (`#1F1F1F`), the theme's own `invalid` colour, and no hint background with the
theme's own comment colour as the foreground.

**A palette change does not leave old rules behind.** `nvs.highlighting.distinctNames` writes
`method:nvs: #FFC66D` into a user's settings. `withoutRules` (`editors/vscode/src/colours.ts:59`)
removes only a rule whose value is exactly what the setting writes now, so after the change the old
rule would count as the user's and stay forever. `colours.ts` therefore keeps the palettes the setting
used to write (`RETIRED`, today one entry: dark method `#FFC66D`). `withoutRules` removes a rule whose
value is the current palette's or a retired one's. `theme.ts` already runs `withoutRules` before
`withRules` (`editors/vscode/src/theme.ts:65`), so a user with the setting on gets the new colour at
the next activation and loses no rule of their own. A retired entry is kept, never removed: a user can
update from any older version. Cost: one more comparison per rule, at activation only.

**The new setting.** `nvs.inlayHints.parameterNames`, a boolean, default `true`, scope `resource`.
It is read by the server, like `nvs.codeLens.enable`:

- `Settings` (`crates/nvs-lsp/src/settings.rs:58`) gains `parameter_names: bool`, read at
  `["inlayHints", "parameterNames"]`. `Changed` gains `inlay_hints`.
- `inlay_hints` (`crates/nvs-lsp/src/server.rs:2707`) drops every hint of kind `PARAMETER` when the
  setting is off. Type hints (`: T` after a `var`) stay: they are the other half of the closed hint set
  and nobody asked to lose them. `hints::for_document` does not change, so the type phase stays the one
  source of a hint.
- A change sends `workspace/inlayHint/refresh` when the client said it supports it
  (`workspace.inlayHint.refreshSupport`), read the way `refresh_lenses` is
  (`crates/nvs-lsp/src/settings.rs:188`) and sent the way `configure` sends the lens refresh
  (`crates/nvs-lsp/src/server.rs:777`). No restart, which keeps the index warm.
- The name is frozen once shipped (`rule:ide/contributions-are-frozen-and-only-ever-added`), so it is
  spelled the way VS Code's own settings group hints: `inlayHints.*`. A later type-hint switch would be
  `nvs.inlayHints.types`. That one is not added now: nobody asked for it.

**Declined:**

- **Turning VS Code's bracket colouring off in `configurationDefaults`.** That changes a user's editor
  under every theme, which `rule:ide/novis-ships-names-not-colours` forbids. Under another theme the
  user keeps VS Code's default.
- **Hints off by default.** A literal argument is the one place a reader cannot see which parameter it
  fills. The theme now draws hints as dim text, and the setting is one click away.
- **A line height or a font in the theme.** A theme cannot set either. `configurationDefaults` could,
  and it is the user's editor.
- **Changing `///` to `/** */` doc comments.** Doc comments already use PhpStorm's colour. The PHP file
  looks lighter because its docblocks have mostly empty lines, not because of the marker.

**What this goal spends.** Nothing on the request path, and no memory. One boolean per server.
Activation compares each written rule with one more palette entry.

**Lanes.** Both stages edit `editors/vscode/test/contributions/contributions.test.ts`, so they run in
order. The goal is small enough that stage 2 may follow stage 1 in the same session.

## Stages

### 1. the Novis themes draw brackets plainly, methods and variables in PhpStorm's colours, and hints without a box

**Does:** changes the two theme files and the palette as § *Standing decisions* says, removes rules an
earlier palette wrote, and updates the tests and docs that state the colours.

- [ ] Novis Dark: the method, function, variable and parameter colours, in both `tokenColors` and
  `semanticTokenColors`, plus the bracket and inlay hint colours.
  `editors/vscode/themes/novis-dark-color-theme.json:1`
- [ ] Novis Light: the bracket and inlay hint colours. `editors/vscode/themes/novis-light-color-theme.json:1`
- [ ] `PALETTE.dark.method` becomes `#56A8F5`, with `RETIRED` and `withoutRules` as § *Standing
  decisions* says, and the module doc says why a retired value is kept. `editors/vscode/src/colours.ts:16`
- [ ] The contributions suite gets the test `draws every bracket level in the theme's text colour, and
  a parameter hint without a box`, for both themes. `editors/vscode/test/contributions/contributions.test.ts:560`
- [ ] The colours suite gets the test `removes a rule the setting wrote with an earlier palette`, and
  a rule with a value in no palette still stays. `editors/vscode/test/client/colours.test.ts:1`
- [ ] The rule's *Built* paragraph names the bracket and hint colours and the retired palette.
  `docs/rules/ide/novis-ships-names-not-colours.md:50`. The reference page needs no change unless it
  names a colour value: check `docs/reference/tools/40-editor.md:450`.
- [ ] Run the `themes-calm` check's command once: green.

### 2. nvs.inlayHints.parameterNames turns the parameter-name hints off, and the server notices without a restart

**Does:** adds the setting to the extension, reads it in the server, filters the hints and refreshes
them on a change, and documents it.

- [ ] Contribute `nvs.inlayHints.parameterNames` after `nvs.codeLens.enable`. Its description:
  "Shows the parameter name before a literal argument, such as `length:` before `4`."
  `editors/vscode/package.json:175`
- [ ] Read it in `Settings::read` and `from_initialize`, add `Changed::inlay_hints`, and read
  `workspace.inlayHint.refreshSupport` beside `refresh_lenses`. The module doc's list of settings says
  what a change to it does. `crates/nvs-lsp/src/settings.rs:58`
- [ ] Filter `PARAMETER` hints in `inlay_hints`, and send `workspace/inlayHint/refresh` from
  `configure` when the setting changed. `crates/nvs-lsp/src/server.rs:2707`
- [ ] Write the two tests the `hints-setting` check names, beside
  `every_setting_is_read_off_initialization_options`. Add `"inlayHints": { "parameterNames": false }`
  to that test too. `crates/nvs-lsp/tests/handshake.rs:345`
- [ ] Add the setting to the client's list of server-read settings. `editors/vscode/test/client/settings.test.ts:27`
- [ ] Rules: add the name to the roster of server-read settings
  (`docs/rules/ide/contributions-are-frozen-and-only-ever-added.md:12`), and say in
  `docs/rules/ide/the-request-set-is-closed.md:1` that the setting removes the parameter-name hints
  and keeps the type hints. Then `bun nv rules --render` and `bun nv rules --check`.
- [ ] Docs: a row in the reference table and the server-read list
  (`docs/reference/tools/40-editor.md:462`, `docs/reference/tools/40-editor.md:479`), and a row in the
  website's settings table (`website/src/content/docs/tools/editor/index.mdx:187`). The website row
  follows AGENTS.md § *Text an end user reads*: "Shows the parameter name before a literal argument.
  With `false`, only the type hints stay."
- [ ] Run the `hints-setting` check's command once: green.

### 3. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. The goal comes from the user's screenshot comparison on 2026-11-04. § *Why*
holds the measured differences and the sampled PhpStorm colours, so no session needs the screenshots.

### Next

Stage 1, from its first item: Novis Dark's colours in
`editors/vscode/themes/novis-dark-color-theme.json:1`. The contributions suite's palette assertion
(`editors/vscode/test/contributions/contributions.test.ts:576`) goes red until `PALETTE.dark.method`
changes with the theme, so change both in one slice.

### Backlog
