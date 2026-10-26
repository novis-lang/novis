Every `file:line` the viewer shows is a link that opens that line in the developer's editor, through
the editor's own URL scheme, and VS Code's `vscode://file/` is the default.

The viewer's settings offer presets for VS Code, VS Code Insiders, VSCodium, Cursor, Zed and the
JetBrains IDEs IntelliJ IDEA and PhpStorm, and a template with `{file}`, `{line}` and `{column}` for
any other editor. A template's scheme is never one that runs or embeds content in the viewer's page
(`javascript:`, `data:`, `vbscript:`, `blob:`): the service refuses to save one, and the viewer builds
no link from one. A list of path mappings rewrites a prefix before the link is built, the longest
that matches, for a program that ran in a container, a virtual machine or WSL and recorded paths the
editor's machine does not have. The setting is the developer's, not the project's, so it lives in the
viewer's settings file and never in `nvs.toml`.

A link is built only from a site the record carries as structured data — a frame, a query's call site,
a dump's or a log call's location — and never from text found inside a message, so a logged string
cannot make a link. A profile the timeline names opens in VS Code's *View Profile* view through the
extension's own `vscode://novis-lang.nvs/` link when the editor is VS Code, and is offered as a download
otherwise.

**Declined:**
- **Opening the file from the service with a command line.** The service would run a program a
  browser asked it to run, which is exactly what a local listener must never do.
- **Linking `file:line` text found in a log message.** Its path is whatever the message says, and the
  structured site is already there for every record that has one.
