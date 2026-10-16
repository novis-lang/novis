The extension's `package.json` declares `extensionKind: ["workspace"]`. It spawns `nvs lsp`, which must
be the binary next to the code — in a WSL distro, over SSH, or inside a devcontainer — and a workspace
extension runs where the code is rather than where the editor's window is.

That one line is the difference between working in every remote configuration and failing in all of
them with a message about `nvs` not being on `PATH`. The same contributions test that checks the frozen
identifiers asserts it.
