The extension ships no `nvs` binary of its own, and a machine without one is offered a guided install
rather than told about `PATH`. The client resolves three candidates in order — `nvs.path`, then the
platform's own lookup, then a copy this extension installed — and uses the first that answers.

When none of them answers, the status item says so and two commands become the way out.
`nvs.downloadBinary` fetches the newest release sharing the client's own `major.minor` series,
because `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` will refuse anything else at
`initialize`, and a fetch that ends in a refusal is a worse first run than no offer at all.
`nvs.openReleases` opens the release page, which is where a user who would rather check the bytes
themselves goes — `gh attestation verify` against the Sigstore provenance every archive carries is a
stronger proof than the extension is willing to build a second trust store to reach.

**The fetch is verified and it is never automatic.** The archive and the release's own `SHA256SUMS`
are taken from the same release, the archive is hashed before anything is unpacked, and a mismatch
aborts, keeps nothing and names the file that failed. Nothing reaches the network unless a user
invoked one of the two commands: no fetch on activation, no background update check, no retry.

**A managed copy is last, and it never writes `nvs.path`.** A binary the extension installed must not
outrank a toolchain the user installed, because the server's answer is a claim about whether the code
compiles — analysing with one `nvs` while the terminal runs another reports on a program nobody will
run. That is the invariant `extensionKind: ["workspace"]` already protects. Writing the path into
settings would defeat it later by other means: the day that user installs `nvs` properly, a stale
absolute path silently keeps winning, so the copy is remembered in the extension's own storage and
the status item names whichever candidate answered.
