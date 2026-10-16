The VS Code extension is what you install to write Novis in VS Code.

It colours a `.nvs` file when the file opens. It then starts `nvs lsp` and shows what that server
returns: errors, completions, documentation and the place where a name is declared. The extension
is a `.vsix` file. It is not on a marketplace, so you install it from that file.

The extension uses the colours of your theme and adds none. `nvs.path` is the path to the `nvs`
binary. When it is empty, the extension finds `nvs` on `PATH`. When `nvs.lsp.enable` is `false`, no
server runs and the file has only its basic colours. The command **Novis: Restart Language Server**
stops the server and starts it again.

**Good to know:** the editor blurs a value that is written to a `secret` variable. Put the cursor
in the value to read it. The blur is only in the editor window. Search results and the diff view
show the real text, and a copy gives the real text too. `nvs.secrets.redact` turns the blur off.
