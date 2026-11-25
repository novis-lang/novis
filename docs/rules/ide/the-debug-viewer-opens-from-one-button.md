The VS Code extension opens the debug viewer with one button, in a panel of the editor or in the
developer's browser, and the panel is a frame around the viewer the service already serves.

**Two commands and one setting.** `nvs.openDebugViewer` (*Open Debug Viewer*) and
`nvs.openDebugViewerInBrowser` (*Open Debug Viewer in Browser*), and `nvs.debugViewer.openIn`,
`editor` by default or `browser`, which decides what the button does. The button is the first command
in `editor/title` for an `nvs` or `nvst` document and for `nvs.toml`, with the `pulse` icon. A second
click reveals the open panel rather than opening another.

**The panel is a frame.** It is one webview, `nvs.debugViewer`, whose page is one `<iframe>` of the
embed link and one nonce script that passes the three messages of
`rule:testing/the-viewer-is-framed-only-by-the-editor-under-a-token-path`. Its content security policy
admits that one frame source and that one script, and nothing may connect. The panel draws nothing of
its own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). A file:line clicked in the
panel arrives as `open`, and the extension opens it in the editor when the file exists under a
workspace folder. A command that opens a path in the viewer, such as a lens, sends `go` to an open
panel, so no second tab opens.

**The extension learns the link from the language server,** which already knows the data folder and
`[debug] listen`. The request `nvs/debugViewer` answers `url`, `embed`, `running` and `recording`, as
`nvs debug link --json` does. When no service runs, the command offers *Start nvs debug*, which runs
`nvs debug` as a task of the `nvs` task type. When `[debug] store` is off, it says which line of
`nvs.toml` turns recording on and opens the file. The browser command passes the link through
`vscode.env.asExternalUri`, so a remote window opens the forwarded address.

**The stop rule.** If the host test shows that VS Code does not draw an `http://127.0.0.1` frame in a
webview, the panel is dropped after that one attempt, both commands open the browser, and this rule
says so.

- **Memory:** one webview per window while the panel is open, holding a frame of the viewer. The
  viewer's own memory is the browser's, as in a tab.

**Declined:**
- **A viewer drawn by the extension.** It would be a second copy of the viewer to keep equal to the
  first.
- **A second button for the browser.** The setting decides what the one button does, and the browser
  command is in the command palette.
- **Starting the service without asking.** A service binds a port and reads the spool, and the
  developer chooses when that happens.
