A setting name lives in somebody's `settings.json` and a command id in their keybindings, so renaming one
later breaks a user's configuration silently. The identifiers are therefore public API, frozen on first
contribution, and anything added later is added, never renamed. A setting leaves the roster only with
the feature it configures, and its name is never given to another setting.

The roster is the `SETTINGS` and `COMMANDS` lists in
`editors/vscode/test/contributions/contributions.test.ts`, which asserts that `package.json` declares
exactly what they name and that every contributed command is registered. A new identifier is added to
those lists and to the manifest in one change; the rule that brings it names it. Among them are
`nvs.path` (the binary, falling back to `PATH`), the `nvs.lsp.*` settings, `nvs.secrets.redact` and
`nvs.taint.mark` (`rule:ide/reveal-is-explicit-and-window-local`,
`rule:ide/tainted-has-no-default-decoration`), and `nvs.check.scope`, `nvs.codeLens.enable` and
`nvs.checkWorkspace` (`rule:ide/check-scope-defaults-to-the-workspace`). A non-standard request,
such as `nvs/redactions` or `nvs/regions`, is frozen the same way under
`rule:ide/the-request-set-is-closed`.
