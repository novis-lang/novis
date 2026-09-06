Appending bytes after an already-signed Mach-O invalidates its signature. `nvs build --compile`
therefore appends the payload *before* signing and ad-hoc-signs the result by default
(`codesign --sign -`), with a flag reserved for a user-supplied identity later. A macOS bundle passes
Gatekeeper's ad-hoc-signature check out of the box.

Windows (PE) and Linux (ELF) have no equivalent step: the append is the entire build. The signing is a
one-time cost inside the build command, paid by the author at build time and never by the end user —
which is also why a bundle never re-signs itself
(`rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`).
