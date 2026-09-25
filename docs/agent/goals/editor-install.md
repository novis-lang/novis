---
milestone: post-parity
---
# Loop goal 37 — The extension guides an install instead of shipping a binary

A VS Code user who installs the extension before the compiler stops hitting a wall. Today the client
spawns `nvs lsp` and, finding nothing, reports a missing binary and leaves the user to solve it; when
this goal is green it names what is missing, offers to fetch the release matching its own series,
checks those bytes against the release's own `SHA256SUMS`, and offers the release page to anyone who
would rather check for themselves. The binary it installs is the **last** thing tried, after
`nvs.path` and `PATH`, so a user's own toolchain always wins.

It sits at the end of the chain because nothing depends on it: goal `editor` shipped a working extension
for people who already have `nvs`, and this goal only widens who that is. It is last rather than
never because the dead end it removes is the first thing a new user meets.

Goal `serve-runs-the-queue`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

It is on the chain at all because the dead end it removes — install the extension, get told about
`PATH`, and stop — is the first thing a new user meets, and the version mismatch of
`rule:ide/the-extension-refuses-a-binary-it-does-not-understand` ends the same way. ADR 0155 is its
whole design and landed before it.

## Stage 0 — the catch-up

Nothing. No fixture predates the rule: `editors/vscode` holds no install path at all today, and the
one sentence that will read differently afterwards — `version.ts`'s refusal, which currently ends
*"Point `nvs.path` at a matching binary"* — is stage 4's to re-point, because there is nothing to
offer until stage 3 exists.

## Stage 1 — the floor

Goal `serve-runs-the-queue`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: the platform table, pinned to the release matrix

The table that turns a machine into an archive name, and the test that stops it drifting from the
workflow that builds those archives. Everything later is a fetch of whatever this decides, so it goes
first and it is provable with no network at all.

1. **`editors/vscode/src/install.ts` — the target table.** Every `(process.platform, process.arch)`
   pair the extension claims, onto one of the seven names
   `.github/workflows/release.yml:205` builds: `linux-x86_64`, `linux-aarch64`, `linux-x86_64-musl`,
   `windows-x86_64`, `windows-aarch64`, `macos-x86_64`, `macos-aarch64`. An unsupported pair resolves
   to nothing and is a case stage 4 reports, not a throw.
2. **musl detection**, in the same file. `process.report.getReport().header.glibcVersionRuntime`
   absent means musl, which is what an Alpine devcontainer needs; the extension is
   `extensionKind: ["workspace"]`, so this runs on the remote and the remote's libc is the one that
   matters.
3. **The archive name and the series selector.** `nvs-<version>-<name>.tar.gz`, `.zip` on Windows,
   from `tools/release.py --package`'s own shape; and given a list of releases, the newest whose
   `major.minor` equals the client's own, per `editors/vscode/src/version.ts:@series`.
4. **`crates/nvs-lsp/tests/extension_release.rs` — the pin.** Reads `install.ts`'s table and
   `.github/workflows/release.yml`'s matrix and fails when either gains a target the other does not
   have. It is Rust rather than TypeScript for one reason: reading the workflow means a YAML parser,
   and a new dependency is exactly what `contributions.test.ts`'s allowlist refuses. This is the
   shape `crates/nvs-lsp/tests/extension_reference.rs` already uses to pin the chapter to the
   manifest.

## Stage 3 — the fetch, and what it refuses

1. **The download**, in `install.ts`: the archive and the release's `SHA256SUMS`, both from the same
   release, over HTTPS to `github.com` and the `objects.githubusercontent.com` hop a release asset
   redirects to.
2. **The verification.** Hash the archive, compare against its `SHA256SUMS` line, and unpack nothing
   until it matches. A mismatch aborts, removes what it wrote, and names the file that failed.
3. **The unpack**, into `context.globalStorageUri`, with the executable bit set on unix — a mode
   inside a zip is not something to rely on.
4. **The suite**, `editors/vscode/test/install/`, printing `install:` through
   `scripts/headless.mjs`'s discovery. No network: the fetch takes an injected transport, and the
   cases feed it a fixture archive, a good `SHA256SUMS` and a tampered one.

## Stage 4 — the chain, the two commands, and the status item

1. **The resolution order** in `editors/vscode/src/extension.ts:98` — `nvs.path`, then `PATH`, then
   the managed copy — replacing the two-way choice there now. The managed copy is remembered in the
   extension's own storage and **never written to `nvs.path`**.
2. **`nvs.downloadBinary` and `nvs.openReleases`**, registered beside the three at
   `editors/vscode/src/extension.ts:62` and added to the frozen roster at
   `editors/vscode/package.json:119`. Added, never renamed.
3. **The offer.** No candidate resolves: the status item says so and the two commands are how a user
   acts on it. Nothing fetches on activation and nothing checks for updates in the background.
4. **The refusal gains a way out.** `version.ts`'s mismatch sentence offers the matching install
   rather than ending at *"Point `nvs.path` at a matching binary"* — which is what makes
   `rule:ide/the-extension-refuses-a-binary-it-does-not-understand` actionable instead of a support
   question.
5. **The status item names which of the three answered**, because with more than one candidate
   "which `nvs` is this" has to be answerable without guessing.

## Stage 5 — the reference chapter

`docs/reference/tools/40-editor.md` gains the install section: the three candidates and their order,
the two commands, what is verified, and the fact that nothing is fetched unasked. Its § *What it does
not do* keeps naming the contributions that are still frozen-and-unanswered — this goal does not
close those, and the playbook bullet at `docs/agent/playbook.md:1261` stays until something does.

## Standing decisions

- **No bundling, and this is not reopened by a session finding the download awkward.** Seven
  platform-specific `.vsix` packages were considered and rejected in [ADR 0155](../../decisions/0155.md)
  § *Alternatives rejected*: a bundled copy either outranks the user's own toolchain, which breaks the
  invariant `extensionKind: ["workspace"]` exists to protect, or it does not, in which case it is this
  goal's managed install with seven publish legs paid for it.
- **Verification is not optional and not deferred to a later stage.** Stage 3 lands the hash check in
  the same slice as the fetch. There is no intermediate state where the extension downloads an
  executable it has not checked, not even behind a flag.
- **Nothing reaches the network without a command.** No fetch on activation, no update check, no
  retry loop. A session that finds this inconvenient writes a better message, not a background task.
- **The managed copy never writes `nvs.path`.** [ADR 0155](../../decisions/0155.md) § 6 is the reason:
  a path the extension wrote outlives its purpose and silently defeats a real toolchain installed
  later.
- **This goal opens ADR 0155**, whose `changes:` block creates
  `rule:ide/the-extension-guides-an-install-and-never-bundles-one` and modifies
  `rule:ide/the-extension-runs-where-the-binary-is`. Both landed with the record; this goal
  implements them and opens no further number.
- **The Sigstore attestation is not verified in-process.** It needs a Sigstore client and a second
  trust store inside an editor extension. `nvs.openReleases` is how a careful user reaches it, and
  ADR 0155 § 5 is where that was decided.
- **Where ambiguity resolves:** an unsupported platform, a release list with no matching series, and
  a network failure all resolve the same way — the status item says which of the three it is, the
  release page stays one command away, and the extension keeps working as a grammar-only client.
  None of them is `BLOCKED`.
- **Not in this goal:** Marketplace publishing (still [ADR 0099](../../decisions/0099.md)
  *Revisiting*'s), a version pin or any multi-version switching ([ADR 0155](../../decisions/0155.md)
  *Revisiting*), and the three frozen-and-unanswered commands from goal `editor`.
