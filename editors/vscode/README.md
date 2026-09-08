# Novis for VS Code

Novis language support for `.nvs` files: TextMate colour the instant a file opens, and everything else
from `nvs lsp` — diagnostics, completion, hover, navigation, semantic colour, formatting.

The extension is a thin client. It holds no parser, no formatter and no type table, and
`rule:ide/one-server-two-thin-clients` is why: language smarts have one implementation, and a second one
arriving here as an npm dependency is what the contributions suite's allowlist refuses.

## Working on it

```
npm ci                      the lockfile is committed; npm ci needs it
npm run compile             tsc into out/
npm run lint
npm run test:headless       every suite under out/test/, no editor and no display
npm run package             the .vsix
```

`python tools/verify.py` from the repository root runs `test:headless` as its last step, after the Rust
gates. The extension-host suite — the tier that needs a running VS Code — is CI's, on Linux under
`xvfb-run`; `rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone` is where that split is
decided.

## Decided here

These are small enough to belong to the extension rather than to a numbered decision record.

**No pixel tier.** Driving the real editor with Playwright to assert that a decoration was *drawn* is
rejected, and not on cost grounds. Anything needing a display sits outside the tier the unattended loop
gates on, so it buys no check where checks are read. What is left once the server's range test, the
position conversion's unit test and the reveal state machine's logic test have run is a CSS constant that
never varies — the test that never fires.

**Nothing is published.** CI builds an installable `.vsix` as an artifact. There is no Marketplace
publisher, no listing and no branding; `rule:ide/one-server-two-thin-clients` § *Revisiting* keeps that
question open.
