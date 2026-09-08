The extension id is `novis-lang.nvs`, the publisher half being the GitHub organisation and the name half
the binary's. The publisher is a Marketplace *identifier* — `vsce` holds it to
`/^[a-z0-9][a-z0-9-]*$/i` — so the project's domain `novis-lang.org` cannot be spelled there and is the
`homepage` instead; a dot would in any case leave `publisher.name` with no unambiguous seam.
The language id is `nvs`, and `extensionKind` is `["workspace"]`. The
client spawns `nvs lsp`, which has to be the binary next to the code, so a WSL distro, an SSH host and a
devcontainer all get the remote's toolchain rather than a missing one.

CI produces an installable `.vsix` artifact. Nothing is published — no Marketplace publisher and no
listing; that decision is open and M4B does not close it. The package does carry the branding a listing
would need. `media/novis-logo.png` is the extension's icon and both languages' file icon, copied from
`website/media/` because a `.vsix` holds no path out of its own directory; `displayName` is the H1 of the
repository's `README.md`, the same home `nvs --help`'s first line reads from
(`rule:packaging/the-banner-states-the-build`), and the extension's own suite asserts it against that
file rather than agreeing with it by hand. `editors/vscode` is a TypeScript package outside the Cargo
workspace.

The `.vsix` carries no binary either, on any platform:
`rule:ide/the-extension-guides-an-install-and-never-bundles-one` is how a machine without `nvs` gets one,
and why a copy the extension installed is tried after the user's own toolchain rather than before it.
