---
looper: 1
title: The website replays what the editor does
summary: "A page shows a recorded editor session that plays like the VS Code extension, from what the real language server returned, and `bun nv site --check` fails when the server returns something else."
context: [tools/nv/cmd/site.ts, website/src/components/Snippet.astro, website/config/tainted-ranges.mjs]
stages:
  - id: 1
    title: the rule and the replay format
    status: done
    extra:
      context:
        rules: [testing/website-pages-are-stamped, ide/a-completion-row-reads-as-a-declaration, ide/completion-is-asked-where-a-spelling-ends]
        shapes: [A rule fragment]
  - id: 2
    title: the recording comes from the language server
    status: done
    context: [tools/nv/lib/lsp.ts]
  - id: 3
    title: the replay plays like the editor
    status: done
    context: [tools/nv/test/html-later-polyfill.test.ts]
  - id: 4
    title: the first replay is on the editor page
    status: open
    context: [website/src/content/docs/install/editor.mdx]
    extra:
      context:
        rules: [testing/website-pages-are-stamped]
checks:
  - id: rules-check
    stage: 1
    name: the rulebook renders and every record is valid
    run: [bun, nv, rules, --check]
  - id: replay-recording
    stage: 2
    name: "a replay's recording is what `nvs lsp` returns for its steps, and `--check` names one that is stale"
    run: [bun, test, tools/nv/test/site-replay.test.ts]
    tests: [a completion step records the list the server returned, a diagnostic step records the error the server published, a stale recording is named by the check]
  - id: replay-browser
    stage: 3
    name: "the replay plays, pauses, steps and stays out of the keyboard order in a real browser"
    run: [bun, test, tools/nv/test/replay-browser.test.ts]
    tests: [a replay starts when it scrolls into view and pauses when it leaves, reduced motion shows the last frame and does not play, only one replay plays at a time, the code in a replay never takes keyboard focus, the list narrows as letters are typed]
  - id: site-snippets
    stage: 4
    name: every snippet and every replay on the website matches the binary
    run: [bun, nv, site, --check, snippets]
  - id: owners-closes
    stage: 4
    name: no designed rule or gap names this goal
    run: [bun, nv, owners, --closes, editor-replay]
    reach: true
---

# The website replays what the editor does

## Why

The website shows code, output and compiler errors, but nothing it shows moves. The language server is
one of the main reasons to choose Novis, and a reader cannot see it from a static page. This goal adds
one reusable component that plays a short editor session: typing, the completion list, a pick, a red
underline and its hover. It plays like the VS Code extension, and everything the server says in it was
recorded from the real `nvs lsp`, so it cannot drift from what Novis does. It comes before
`property-paths`, whose docs stage is built on it, so that goal gets a finished tool rather than
building two things at once.

## Standing decisions

**The pattern is the `.tainted` sidecar's.** `bun nv site --bless` already drives `nvs lsp` for a
snippet and writes what it returned beside it (`tools/nv/cmd/site.ts:62`, `tools/nv/cmd/site.ts:876`),
and `Snippet.astro` draws it (`website/src/components/Snippet.astro:54`). A replay is the same: a
hand-written `.replay` beside a snippet lists the steps, `--bless` writes the recording as
`.replay.out`, and `--check snippets` fails when the server now returns something else. The format's
home is `tools/nv/cmd/site.ts` § *A snippet*, and this goal restates none of it once it is written there.

**The snippet `.nvs` is the last frame.** The `.replay` names the text the session starts from and the
steps that turn it into the snippet. A replay whose steps do not end on the snippet's text is a
`--check` failure, so the page's code and the replay never disagree.

**Steps, and only three kinds at first:** type text (one character at a time), accept the selected
completion row (Enter or Tab), and show the diagnostic under the cursor (hover). A completion list is
recorded wherever the server would be asked: at a trigger character
(`rule:ide/completion-is-asked-where-a-spelling-ends`) and at the first letter of a word. Hover cards
and quick fixes are added when a page needs them, not before.

**It imitates the editor.** Typing at a human pace with a blinking text cursor; the completion list
where VS Code opens it, each row as `rule:ide/a-completion-row-reads-as-a-declaration` describes it,
with its kind icon; the list narrowing as letters are typed, the matched letters highlighted and the
selection moving; the accepted name written into the code; a red squiggle and its hover for an error;
an editor frame with a tab title, line numbers and the website's light and dark colours.

**What is recorded and what is imitated.** The server's answers are recorded: each list, each
diagnostic, each hover. Narrowing a list as letters are typed is VS Code's own work, not the server's,
so the component does it: case-insensitive matching of the letters in order, best match first, matched
letters highlighted. The component's head comment says which parts are recorded and which are
imitated. It is never presented as VS Code's exact behaviour.

**It is clearly a demo.** A "Recording" badge in the frame's title bar, and a caption above it: "A
recording from the Novis editor extension. You cannot type here.", linking to the editor install page.
The play, pause, previous and next buttons sit outside the editor frame. Clicking the code does
nothing, the code never takes keyboard focus, and the mouse shows no text cursor over it.

**How it plays.** It starts when at least half of it is in view, pauses when it leaves, plays once and
stops on the last frame with a "Play again" button. Play and pause are always visible and work from
the keyboard, and previous and next step one keystroke. With `prefers-reduced-motion` it does not
start: it shows the last frame with its list open and a play button. Only one replay plays at a time
on a page. Below it, a collapsed "What this recording shows" section gives the same steps as text, for
screen readers and search.

**One component.** `website/src/components/Replay.astro`, used as `<Replay src="..."/>` beside
`<Snippet>`, with its script in the same file. No editor library is loaded: the frame, the list and
the squiggle are plain HTML and CSS, and the code is coloured the way `<Snippet>` colours it. The page
weight is the recording and one small script.

**The rule** is the new `testing/a-replay-plays-what-the-language-server-returned`, beside
`rule:testing/website-pages-are-stamped`, written in stage 1 as `designed` and owned by this goal. It holds the decisions above that a later page
author must keep: recorded from the server, checked by `--check`, imitated narrowing, the demo label,
the playing rules. Every caption and button label follows AGENTS.md § *Text an end user reads*.

**Trade-offs.** Nothing changes in the language or the binary. The website gains one script and a
recording per replay. `--bless` and `--check snippets` start `nvs lsp` once more per replay. A replay
fails `--check` when the server's answers change, which is the point of recording it.

## Stages

### 1. the rule and the replay format

**Does:** the rule fragment exists as `designed`, and the `.replay` and `.replay.out` formats are written in the site tool's header.

- [x] The rule fragment and its record, owned by this goal. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md:1`
- [x] The `.replay` and `.replay.out` formats in § *A snippet*. `tools/nv/cmd/site.ts:70`

### 2. the recording comes from the language server

**Does:** `--bless` plays a `.replay`'s steps against `nvs lsp` and writes `.replay.out`, and `--check snippets` names a stale one.

- [x] Drive the steps through the session `nvs lsp` is sent for a snippet. `tools/nv/cmd/site.ts:922`
- [x] `--check` compares the recording and the last frame against the snippet. `tools/nv/cmd/site.ts:1056`
- [x] The tests. `tools/nv/test/site-replay.test.ts:1`

### 3. the replay plays like the editor

**Does:** `<Replay>` plays a recording like the VS Code extension, with the controls, the label and the playing rules above.

- [x] The component, its frame, list, squiggle and controls. `website/src/components/Replay.astro:1`
- [x] The browser tests, on the cached Chromium the polyfill test uses. `tools/nv/test/replay-browser.test.ts:1`

### 4. the first replay is on the editor page

**Does:** the editor install page shows a replay of `as property<T>` completion and a misspelled name's error.

- [ ] The snippet, its `.replay` and its blessed `.replay.out`. `website/snippets/install/property-key-completion.nvs:1`
- [ ] The page uses `<Replay>` and is re-stamped. `website/src/content/docs/install/editor.mdx:1`
- [ ] The rule flipped to `shipped` with its guard tests. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md:1`

## Handoff

### State

Stages 1 to 3 are done, and `replay-recording` and `replay-browser` are green. `<Replay src>`
(`website/src/components/Replay.astro`) reads a snippet's `.replay` and `.replay.out` at build time.
`website/config/replay-frames.mjs` turns them into one frame per keystroke and writes the HTML:
caption, `inert` frame with tab title and "Recording" badge, buttons, the collapsed steps text and
the frames as JSON. It is also the one `.replay` parser, which `tools/nv/lib/replay.ts` imports.
`website/config/replay-player.mjs` plays the frames and narrows lists through `replay-narrow.mjs`.
`tools/nv/test/replay-browser.test.ts` tests the player in Chromium on a fixture, through the shared
CDP driver `tools/nv/test/chromium.ts`, and CI's `browser` job runs it. The Astro build has not
rendered `Replay.astro` yet, because no page uses it. The code in a replay is not coloured yet:
`<Snippet>` colours through Expressive Code, and the player draws plain text.

### Next

**Stage 4: the first replay is on the editor page**, one file set: `website/snippets/install/`,
`website/src/content/docs/install/editor.mdx`, `website/src/components/Replay.astro`.

- [ ] The snippet, its `.replay` and its blessed `.replay.out` (`rule:testing/a-replay-plays-what-the-language-server-returned`). `website/snippets/install/property-key-completion.nvs:1`
- [ ] The page uses `<Replay>` beside `<Snippet>`, `bun nv site --build` renders it, and the page is re-stamped. `website/src/content/docs/install/editor.mdx:7`
- [ ] Colour the replay's code the way `<Snippet>` does, through the grammar in `website/config/novis.tmLanguage.json`, with the site's light and dark colours. `website/config/replay-frames.mjs:204`
- [ ] The rule flipped to `shipped` with its guard tests. `docs/rules/testing/a-replay-plays-what-the-language-server-returned.md:1`

### Backlog

- Hover cards and quick fixes as replay steps, when a page needs them (this file's § *Standing decisions*).
- The run before this session reported 2047 owed floor atoms, red on `docs/examples/config/*` proofs.
  No session has looked at them yet (`bun nv proofs`).
- The nightly CI ran `origin/main`, which is far behind local `main`, and every job of that run was red.
  Nothing here can be fixed until `main` is pushed; recheck on the next pushed run (`.github/workflows/`).
