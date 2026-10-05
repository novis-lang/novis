# Novis for VS Code

Novis language support for `.nvs` files: TextMate colour the instant a file opens, and everything else
from `nvs lsp` — diagnostics, completion, hover, navigation, semantic colour, formatting.
In the Explorer, a file that declares one class, interface, enum or type alias shows the letter `C`, `I`,
`E` or `T` beside its name.

The extension is a thin client. It holds no parser, no formatter and no type table, and
`rule:ide/one-server-two-thin-clients` is why: language smarts have one implementation, and a second one
arriving here as an npm dependency is what the contributions suite's allowlist refuses.

## Working on it

```
npm ci                      the lockfile is committed; npm ci needs it
npm run compile             tsc into out/, then the bundle
npm run bundle              esbuild writes the client and its dependencies into out/extension.js
npm run watch               the bundle, rebuilt on every save
npm run lint
npm run test:headless       every suite under out/test/, no editor and no display
npm run test:host           the same suites' other tier, in a real VS Code
npm run package             the .vsix
```

The extension runs from `out/extension.js`, the bundle, and not from the files `tsc` writes under
`out/src/`. Those are for the test suites. The `.vsix` carries the bundle and no `node_modules/`.

`bun nv verify` from the repository root runs `test:headless` as its last step, after the Rust
gates. `test:host` is the other tier and runs on every acceptance sweep, here and in CI:
`scripts/host.mjs` downloads a pinned VS Code into `.vscode-test/`, opens a copy of `test/host/fixture/`
in a throwaway profile with every other extension disabled, and prints the report the in-host runner
wrote. It opens a window on the desktop for about a minute, and under `xvfb-run` on Linux it does not.
`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone` is where the split between the two
tiers, and the isolation that makes this one safe to run on a machine someone is working on, are decided.

## Decided here

These are small enough to belong to the extension rather than to a numbered decision record.

**No pixel tier.** Driving the real editor with Playwright to assert that a decoration was *drawn* is
rejected, and not on cost grounds — the host tier already runs a real editor on every sweep. It is
rejected because the editor answers every question a pixel would, as data: `test:host` reads the scopes,
the semantic tokens, the published diagnostics and the ranges a decoration was registered over through
VS Code's own API, and asserts them exactly rather than by appearance. What a screenshot adds on top of
that, once the server's range test, the position conversion's unit test and the reveal state machine's
logic test have run too, is a CSS constant that never varies — the test that never fires.

**The test surface `activate` returns is read-only.** The host tier reads almost everything it asserts
through VS Code's own API, but a decoration and a `LanguageStatusItem` are sinks — `setDecorations` takes
ranges and hands none back — so what is concealed on screen and what the status item says can only be
observed if this extension says so. `activate` returns the three readings in `src/surface.ts` and nothing
else: the status item's text and severity, the AST view's provider, and the ranges each visible editor was
last handed per decoration kind. There is no reveal on it, no setter and no way to run anything, because
`activate`'s return value is `extension.exports` and every extension installed beside this one can reach
it: a reveal there would uncover a `secret` on the user's screen without the user asking, which
`rule:ide/reveal-is-explicit-and-window-local` reserves for the person at the keyboard. It holds no
language logic either, so `rule:ide/dependencies-are-allowlisted` is untouched by it.

**The `tainted` marker is a themed text glyph, not a codicon.** [ADR 0101](../../docs/decisions/0101.md)
§ 4 asks for a themed codicon after each `tainted` declaration, and the API it would be drawn with does
not offer one: an inline decoration attachment takes `contentText` **or** an image path and never both,
and only the text half takes a `ThemeColor`. An image would therefore ship a colour of Novis's own into
the user's theme, which `rule:ide/novis-ships-names-not-colours` refuses more strongly than § 4 asks for
the icon. So the glyph is one BMP geometric character — no emoji presentation, no colour font to be at
the mercy of, drawn in the editor's own face and coloured by a theme reference. The record's reasoning
is met; its noun is not.

**Nothing is published.** CI builds an installable `.vsix` as an artifact. There is no Marketplace
publisher, no listing and no branding; `rule:ide/one-server-two-thin-clients` § *Revisiting* keeps that
question open.

**One version, compared by series.** The extension's `version` is the workspace's, from the root
`Cargo.toml`, and the client refuses a server outside its own `major.minor`
(`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`, `src/version.ts`). A second constant
naming the server versions understood was the alternative, and it is one more thing to forget: the two
halves ship from one commit, the contributions suite asserts the manifest and the workspace agree, and
the protocol suite asserts the binary on disk is in the series. A patch release is accepted either way
round, because it fixes answers rather than changing the shape of the protocol.

**Everything runs from a copy of the binary, never from the binary.** A process started from the file
`nvs.path` names holds that file for as long as it runs, and a build or an upgrade then cannot write it.
`src/shadow.ts` copies the binary into the extension's storage and hands the copy over only once both
files hash equal; `src/binary.ts` is asked for it at every spawn — the server, the Tasks, the formatter,
the AST panel, the test controller — and reads the binary's stamp each time, so nothing is started from
an older build than the one on disk. A new build takes over the server within seconds, started beside
the old one and switched to with no gap (`rule:ide/the-extension-runs-a-copy-of-the-binary`). The status
item's detail says which build is answering.

**The version is read at `initialize`, so the refusal is a stop and not a non-start.** LSP has nowhere
earlier to report one, and asking `nvs --version` first would be a second process and a second answer to
keep in step. The client starts the server, reads `serverInfo`, and stops it before it is handed a
document or a request — what the rule refuses is a session.
