# Handoff

## State

**Goal `editor-install` — stage 3 is landed whole and stage 4 is untouched.** `install.ts` is now the
offer end to end: which archive this machine needs, which release it comes from, a download that
returns no bytes the release's own `SHA256SUMS` vouches for, and an unpack into a directory the
caller names. Nothing calls any of it yet — stage 4 is the commands and the resolution chain.

`editors/vscode/test/install/` exists and prints `install:` through `scripts/headless.mjs`, which
discovers it without an `ORDER` entry. 184 passing headless, one pending: the executable-bit case
skips itself on Windows, where the assertion has no meaning.

**The unpack dependency question is answered, and the answer needs no new package.** `node:zlib`
plus a 512-byte tar header walk and a zip central-directory read, both in `install.ts` — so
`contributions.test.ts`'s dependency allowlist is untouched and no `child_process` extractor has to
exist on the machine. The archive layout the reader depends on is stated in `unpack`'s own doc
comment, which is the home for it; `tools/release.py:469` is the other end.

The transport is a parameter (`install.ts`'s `Transport`), `overHttps` is the only thing that opens a
socket, and no case calls it. Design is `rule:ide/the-extension-guides-an-install-and-never-bundles-one`
and [ADR 0155](../decisions/0155.md) § 5. Nothing is blocked.

**The pack's gap, for the record:** `[context.stage.3]` carried `adrs` only, and this stage had to
read two *regions* of files a `modules` entry prints one line of — `tools/release.py:469-515` for
what is inside an archive, and `.github/workflows/release.yml:410-414` for what `SHA256SUMS` looks
like. A stage whose work turns on a region of a non-module file has no manifest field for it; a
handoff anchor is the only lever, so stage 4's items below carry theirs.

## Next group

**Stage 4: the chain, the two commands, and the status item** — one file set:
`editors/vscode/src/extension.ts`, `editors/vscode/package.json`,
`editors/vscode/test/contributions/contributions.test.ts` and `editors/vscode/src/version.ts`.
Design is `rule:ide/the-extension-guides-an-install-and-never-bundles-one` and
`rule:ide/contributions-are-frozen-and-only-ever-added`, with ADR 0155 §§ 2, 3, 6, 7 and 8 —
`[context.stage.4]` already slices all five.

- [ ] **The resolution chain, three candidates deep** — `editors/vscode/src/extension.ts:107`,
      which is the two-way `nvs.path`-or-`PATH` choice today. `nvs.path`, then the platform's
      lookup, then the copy under `context.globalStorageUri` if `installBinary` left one there.
      The managed copy is last and **is never written into `nvs.path`** (ADR 0155 § 6); remember it
      by looking for the file, not by storing a path that can go stale.
- [ ] **The two commands, added to the frozen roster** — registered beside the four at
      `editors/vscode/src/extension.ts:72`, declared at `editors/vscode/package.json:137`, and added
      to `COMMANDS` at `editors/vscode/test/contributions/contributions.test.ts:85`.
      `nvs.downloadBinary` calls `newestInSeries` → `downloadVerified` → `installBinary` with
      `overHttps`; `nvs.openReleases` opens the release page. Added, never renamed
      (`rule:ide/contributions-are-frozen-and-only-ever-added`).
- [ ] **The status item says which candidate answered, and the refusal offers the install** —
      `editors/vscode/src/extension.ts:115` for the `report` call, and
      `editors/vscode/src/version.ts:75` for the mismatch sentence that currently ends at *"Point
      nvs.path at a matching binary"*. An unsupported platform, a series with no release and a
      network failure all end in the status item naming which of the three it is.

## Backlog

- Stage 5: the reference chapter for the two commands — `docs/reference/tools/40-editor.md`.
- `overHttps` is the one function in `install.ts` no case covers; it needs a socket by definition.
- Replacing a running `nvs.exe` on Windows fails with EBUSY; `installBinary` leaves the `.part`
  removed and throws, and stage 4 decides what the command says about it.
