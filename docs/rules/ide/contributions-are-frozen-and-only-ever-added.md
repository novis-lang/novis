A setting name lives in somebody's `settings.json` and a command id in their keybindings, so renaming one
later breaks a user's configuration silently. The identifiers are therefore public API, frozen on first
contribution, and anything added later is added, never renamed. A setting leaves the roster only with
the feature it configures, and its name is never given to another setting.

M4B's roster. Settings: `nvs.path` (the binary, falling back to `PATH`), `nvs.lsp.enable`,
`nvs.lsp.debounce`, `nvs.lsp.trace.server`, and — added under this rule by
`rule:ide/reveal-is-explicit-and-window-local` and `rule:ide/tainted-has-no-default-decoration`
— `nvs.secrets.redact` (default `true`) and `nvs.taint.mark` (default `off`). Commands: `nvs.run`,
`nvs.test`, `nvs.showAst`, `nvs.restartServer`, and from the same source `nvs.revealSecret` and
`nvs.hideSecrets`. Nothing else is contributed at M4B.

M10 adds, under the same rule and not as an exception to it: the settings `nvs.check.scope`,
`nvs.codeLens.enable` and `nvs.template.services`, the command `nvs.checkWorkspace`, and a second
request of Novis's own, `nvs/regions`. M10's PHP-name setting left with the PHP-name completion it
configured. A contributions test asserts `package.json` declares exactly what the roster names.
