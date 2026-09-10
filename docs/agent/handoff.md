# Handoff

## State

**Goal `editor-install` — stage 2 is landed and stage 3 is untouched.** The extension can now say
which archive a machine needs and which release built it, and nothing of the fetch exists yet.

`editors/vscode/src/install.ts` is pure decision: it reads no configuration, touches no disk and
reaches no network, so every part of it is provable offline. It holds `TARGETS` (the seven
`(platform, arch, libc)` rows), `currentLibc`, `targetFor`/`currentTarget`, `archiveName` and
`newestInSeries`. `crates/nvs-lsp/tests/extension_release.rs` pins the table to
`.github/workflows/release.yml`'s `build` matrix — names, archive format and the musl flag — and
reads both files as text, so no YAML parser and no new npm dependency exists to fail
`contributions.test.ts`'s allowlist.

`editors/vscode/src/version.ts` gained `parts()`, which is now the one home for what a release
number looks like: `series()` is its first two fields joined and `install.ts` orders a release list
by the rest. Tags carry a `v` and archives do not, so `releaseVersion` strips it.

**The design is settled** — [ADR 0155](../decisions/0155.md) and
`rule:ide/the-extension-guides-an-install-and-never-bundles-one`, with the four load-bearing calls in
the goal prose's *Standing decisions*. Nothing is blocked.

**The one decision stage 3 owes in its first slice: how it unpacks with no new dependency.** Two
candidates, both inside `contributions.test.ts`'s allowlist because neither adds a package — Node's
own `zlib` plus a minimal tar reader in `install.ts`, or the platform's `tar`/`Expand-Archive`
through `child_process`. Not decided here.

## Next group

**Stage 3: the fetch and what it refuses** — one file set: `editors/vscode/src/install.ts` and a new
`editors/vscode/test/install/`. Design is `rule:ide/the-extension-guides-an-install-and-never-bundles-one`
and ADR 0155 § 5, which `[context.stage.3]` already slices.

- [ ] **The download and its verification, in one slice** — `editors/vscode/src/install.ts:152`,
      under `newestInSeries`. The archive and the release's `SHA256SUMS` from the same release,
      hashed before anything is unpacked; a mismatch aborts, keeps nothing and names the file. The
      transport is a parameter so a case can feed it fixtures — the standing decisions forbid an
      intermediate state where the fetch lands without the check.
- [ ] **The unpack** — same file, into `context.globalStorageUri`, with the executable bit set on
      unix. The archive holds `nvs-<version>-<name>/nvs[.exe]`, which is
      `tools/release.py:482`'s staging read from the other end.
- [ ] **The suite** — `editors/vscode/test/install/` , discovered by
      `editors/vscode/scripts/headless.mjs:23` and printing `install:`. A tampered `SHA256SUMS` that
      must leave nothing on disk, the series selector over a list spanning several series, and the
      table's unsupported-pair answer. No network in any case.

## Backlog

- Stage 4: the three-candidate order, the two commands and the status item —
  `editors/vscode/src/extension.ts:98`; it also re-points `version.ts`'s refusal sentence.
- Stage 5: the install section of the reference chapter — `docs/reference/tools/40-editor.md`.
- The musl leg is `optional: true` in the matrix (`.github/workflows/release.yml:213`), so a musl
  machine can meet a release that has no archive for it; that is stage 4's report, not a table gap.
- No aarch64 musl archive exists, so Alpine on ARM resolves to nothing —
  `editors/vscode/src/install.ts:43` says so where the table is.
