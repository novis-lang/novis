The extension id is `novis-lang.nvs`, the publisher half being the GitHub organisation and the name half
the binary's. The publisher is a Marketplace *identifier* — `vsce` holds it to
`/^[a-z0-9][a-z0-9-]*$/i` — so the project's domain `novis-lang.org` cannot be spelled there and is the
`homepage` instead; a dot would in any case leave `publisher.name` with no unambiguous seam.
The language id is `nvs`, and `extensionKind` is `["workspace"]`. The
client spawns `nvs lsp`, which has to be the binary next to the code, so a WSL distro, an SSH host and a
devcontainer all get the remote's toolchain rather than a missing one.

CI produces an installable `.vsix` artifact, and every release attaches it to the GitHub release as
`nvs-<version>.vsix`, at the release's own version (`.github/workflows/release.yml`'s `vsix` job).

**Publishing a release lists that same file on the VS Code Marketplace.** The `novis-lang` publisher
lists it as `novis-lang.nvs`, and the publisher ID was registered as `Novis-Lang`: the Marketplace looks
publishers up without regard to case, as VS Code does extension ids, so the manifest keeps the lowercase
spelling. `release-promote.yml`'s `marketplace` job downloads the release's `.vsix`, checks its build
attestation, and uploads it, so the listing is the bytes `SHA256SUMS` names and nothing is built twice.
It stores no secret: a Microsoft Entra ID identity that is a member of the publisher and of nothing else
trusts GitHub's OIDC token for the `marketplace` environment alone, and that environment admits `v*`
tags and `main` and waits for a reviewer. The reviewer is the gate because the Marketplace updates every
installed copy without the user acting, so a listing written by anything with push access would reach
every editor that has the extension. A version behind the newest tag is skipped
(`bun nv release --newest`), so publishing an old draft late moves no listing back. `"preview": true`
marks the listing while Novis is before 1.0.

The package carries the branding the listing shows. `media/novis-logo.png` is the extension's icon and both languages' file icon, copied from
`website/media/` because a `.vsix` holds no path out of its own directory; `displayName` is the H1 of the
repository's `README.md`, the same home `nvs --help`'s first line reads from
(`rule:packaging/the-banner-states-the-build`), and the extension's own suite asserts it against that
file rather than agreeing with it by hand. `editors/vscode` is a TypeScript package outside the Cargo
workspace.

The `.vsix` carries no binary either, on any platform:
`rule:ide/the-extension-guides-an-install-and-never-bundles-one` is how a machine without `nvs` gets one,
and why a copy the extension installed is tried after the user's own toolchain rather than before it.

**Declined:**
- **A personal access token for the Marketplace.** It is one secret in the repository, valid for up to
  a year across every Azure DevOps organisation of its owner, and it has to be renewed by hand. The Entra
  ID identity has none of the three.
- **A listing on Open VSX**, the registry VSCodium, Cursor and other VS Code builds read. It has no OIDC
  exchange, so it needs a stored token in the release. Its users install the release's `.vsix`.
- **No reviewer on the `marketplace` environment.** Publishing the draft would then be the only click,
  and any push to `main` or of a `v*` tag could publish a listing every installed copy updates to.
