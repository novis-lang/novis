The extension id is `nvs-lang.nvs`, the language id is `nvs`, and `extensionKind` is `["workspace"]`. The
client spawns `nvs lsp`, which has to be the binary next to the code, so a WSL distro, an SSH host and a
devcontainer all get the remote's toolchain rather than a missing one.

CI produces an installable `.vsix` artifact. Nothing is published — no Marketplace publisher, no listing,
no branding; that decision is open and M4B does not close it. `editors/vscode` is a TypeScript package
outside the Cargo workspace.
