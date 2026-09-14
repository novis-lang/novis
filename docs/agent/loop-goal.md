---
milestone: M4B
---
# Loop goal 56 — the extension is tested in a real editor host and built by CI

M4B's last promises are kept. The VS Code extension is proved inside a real extension host, not only
by the headless tier around it. That covers TextMate colour before the server answers and semantic
colour after, a legend equal to the server's, Tasks whose failures reach the Problems panel, the AST
panel rendering a file that does not compile, and concealment drawn on open. The same host suite runs on
the developer's Windows machine and on Linux CI under `xvfb-run`. CI builds and tests the extension on
all three platforms and produces the installable `.vsix`. The five-minute fuzz run over truncated
input has a local way to run, and a verdict on record.

## Why here

The chain runs one goal per past milestone, in milestone order, to empty every promise a past
milestone's plan made (M0–M8). M4B comes after goal `m5-proofs` only because of that order: this goal
needs nothing from it and shares no files with it. Its file set is `editors/vscode/`,
`.github/workflows/ci.yml`, `tools/ci-changes.py` and `fuzz/`. Before goal `gap-zero` for that goal's
standing reason, and for one specific to this goal: goal `gap-zero` needs a green CI run, and the two CI
jobs added here must be part of it.

What it needs already built:

- the server half of M4B (`nvs lsp-test tests/lsp/` reported 267 passed at the audit that cut this goal)
- the headless tier: `editors/vscode/scripts/headless.mjs` with the grammar, contributions, protocol,
  client, install and surfaces suites
- packaging, which is already a floor check (`vsix packages`, `npm run --silent package`)
- the fuzz target `prefix` (`fuzz/Cargo.toml:37-42`, `fuzz/fuzz_targets/prefix.rs`) and CI's
  `fuzz-smoke` job for it (`.github/workflows/ci.yml:419-458`)

## Stage 0 — the catch-up

Sentences on disk this goal makes wrong. The stage that makes each one wrong corrects it. Re-grep
before editing: these are anchors, and files move.

- `docs/rules/ide/headless-gates-the-loop-the-host-run-gates-the-milestone.md:10-13` says the host run
  is "in CI only" and "not on the loop's acceptance list at all". Stage 2's record amends it.
- In this goal's own `.toml`, the comment over the floor's `vsix packages` check (carried from goal
  `http-client`'s list) says the extension-host suite "must not be added". Stage 2 rewrites that comment
  whole, pointing at the amended rule.
- `editors/vscode/README.md:20-23` says the host suite is CI's. Stage 3 rewrites the paragraph and adds
  `npm run test:host` to *Working on it*.
- `editors/vscode/README.md:29-33`: *No pixel tier* is reasoned from "anything needing a display sits
  outside the tier the unattended loop gates on", and that stops being true. The decision stands, so
  stage 3 rewrites its reason whole. The host tier asserts scopes, tokens, diagnostics and decoration
  ranges as data through the editor's own API, and what a drawn pixel adds on top is still a CSS
  constant.
- `editors/vscode/scripts/headless.mjs:1-9` and `:23-30` run "every suite under `out/test/`". A
  `host` directory there would be picked up and fail on `import "vscode"`, so stage 3 excludes it and
  rewrites the header comment.
- `editors/vscode/test/client/concealment.test.ts:3-6` says the README records why no tier drives
  `setDecorations`. Stage 5 makes the host tier drive it, and rewrites the comment whole.
- `docs/agent/commands.md:613-617` names `lex` and `parse` as the targets to fuzz under WSL, but not
  `prefix`, which is the one M4B's *Verify* names. Stage 7 adds `prefix` and the local command.

## Stage 1 — the floor

Goal `m5-proofs`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the record

One new record, and no other number. Its body is § *Standing decisions* below, argued. It modifies
`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone` and creates no rule:

- **The host tier runs wherever a developer does**: the Windows desktop the loop runs on, and Linux CI
  under `xvfb-run -a`. It is on the acceptance list as `npm run test:host`.
- **Isolation is what makes that safe.** The rule said "CI only" because an unisolated second instance
  attaches to the developer's running editor and exits with no results, and because a test writing a
  setting writes it into the developer's own `settings.json`. A downloaded, pinned VS Code build,
  `--user-data-dir` and `--extensions-dir` under `.vscode-test/`, and a copied fixture folder answer
  both.
- **The rule's title and body are rewritten whole**, and its id stays: renaming a rule under its
  citations is a tree-wide sweep that buys nothing here. The headless half is unchanged.

Then run `python tools/rules.py --render`, and rewrite the floor comment Stage 0 names.

## Stage 3 — the keystone: the host harness

One file set: `editors/vscode/package.json`, `package-lock.json`, `scripts/host.mjs` (new),
`scripts/headless.mjs`, `test/host/` (new), `test/contributions/contributions.test.ts`, `README.md`.

- **`@vscode/test-electron` in `devDependencies`**, with its allowlist entry in the contributions
  suite (`rule:ide/dependencies-are-allowlisted`), and a `test:host` script:
  `tsc -p ./ && node scripts/host.mjs`.
- **`scripts/host.mjs`** calls `runTests` against a pinned VS Code version that is cached in
  `.vscode-test/`. Both platforms run the same script. It passes:
  - `--user-data-dir` and `--extensions-dir` under `.vscode-test/`, and `--disable-extensions`
  - a copy of `test/host/fixture/` as the workspace, never the repository
  - `nvs.path`, written only into that throwaway profile's `settings.json`. The path comes from
    `--nvs <path>`, falling back to the lookup `test/protocol/session.ts:72-84` uses (`NVS_BIN`, then
    `target/`).
- **The in-host index** (`test/host/index.ts`) runs Mocha with the `spec` reporter. It writes the
  report to `.vscode-test/host-report.txt`, ending on `host: N passing, M failing`. `host.mjs` prints
  that file after the editor exits and exits non-zero on any failure. This means the loop reads the
  launcher's own stdout on every platform instead of relying on Electron's.
- **`headless.mjs` skips `out/test/host/`.** The floor's `vscode (headless)` check is what catches a
  mistake here.
- **Its first test** is `runs in a throwaway profile and never the developer's`. It asserts that
  `env.appRoot` is under `.vscode-test/`, that the user-data directory is the throwaway one, and that
  the open folder is the fixture copy.

Every later stage's test is written into this harness.

## Stage 4 — colour and activation, in the host

`editors/vscode/test/host/colour.test.ts` and fixtures under `test/host/fixture/`. No `src/` change is
expected. Each `it` is named exactly as the check wants it, in this order:

1. **`activates on a .nvs file and not on a .php file`**: open `other.php`, assert that
   `novis-lang.nvs` is not active, then open `app.nvs` and assert that it is
   (`rule:ide/the-extension-claims-nvs-only`).
2. **`colours a file from the grammar before the server answers`**: with `nvs.lsp.enable` off in the
   throwaway profile, `_workbench.captureSyntaxTokens` on `app.nvs` returns the grammar's scopes, and
   `vscode.provideDocumentSemanticTokens` returns nothing (`rule:ide/highlighting-is-two-layers`).
3. **`adds semantic colour once the server answers`**: with the setting turned on, the same document
   returns semantic tokens, and a `secret` binding carries the `secret` modifier
   (`rule:ide/semantic-tokens-carry-the-qualifiers`).
4. **`registers the legend the server declares`**: `vscode.provideDocumentSemanticTokensLegend` equals
   the legend `nvs lsp` returns from `initialize`, read through the protocol suite's
   `test/protocol/session.ts`.
5. **`opens a .nvst case coloured`**: `_workbench.captureSyntaxTokens` on `case.nvst` shows the section
   delimiter's scope and Novis's scopes inside `--FILE--` (`rule:ide/case-files-have-their-own-grammar`).
6. **`selects $total whole`**: `document.getWordRangeAtPosition` inside `$total` covers the `$`. This is
   the editor applying `wordPattern`, not the regex alone.

## Stage 5 — the surfaces, in the host

`editors/vscode/test/host/surfaces.test.ts`, `editors/vscode/src/extension.ts:@activate`,
`src/redactions.ts:243-246`, `src/ast.ts:58`, `src/tasks.ts:57`.

- **A read-only test surface.** `activate` returns an object with no method that changes, reveals or
  runs anything. It holds the status item's current text and severity, the AST view's provider, and
  the ranges last handed to `setDecorations` for each editor and decoration kind. It is recorded in the
  README's § *Decided here*.

Each `it` is named exactly as the check wants it, in this order:

1. **`shows the server's health and version in the status item`**: the status item names the binary's
   version (`src/extension.ts:72`).
2. **`runs nvs run as a task whose failure reaches the Problems panel`**: `nvs.run` on `broken.nvs`,
   wait for the task to end, then `languages.getDiagnostics(uri)` holds the `error[E0301]` the
   `problemMatcher` parsed (`rule:ide/tasks-carry-a-problem-matcher`).
3. **`renders the AST panel for a file that does not compile`**: `nvs.showAst` on `broken.nvs` makes
   the view visible, and its provider's root children include the declaration the errors are about
   (`rule:ide/the-ast-panel-shells-out-to-the-cli`).
4. **`conceals both secrets on open and reveals exactly one`**: opening `secrets.nvs` hands both literal
   ranges to the concealing decoration. `nvs.revealSecret` on the first leaves only the second
   concealed (`rule:ide/redaction-ranges-come-from-the-server`,
   `rule:ide/reveal-is-explicit-and-window-local`).
5. **`decorates nothing for tainted at the default setting`**: with `nvs.taint.mark` unset, no taint
   decoration is handed any range (`rule:ide/tainted-has-no-default-decoration`).

## Stage 6 — CI builds, tests and packages the extension

`.github/workflows/ci.yml`, `tools/ci-changes.py:34-64`, and `editors/vscode/.nvmrc` (new).

- **An `editor` lane in `LANES`**: `editors/`, `crates/`, `Cargo.toml`, `Cargo.lock` and
  `.github/workflows/`. The protocol and host suites drive the real binary, so any crate can change
  their answers. It is emitted as an output of the `changes` job (`ci.yml:70-76`).
- **Job `extension`**, a matrix over the same three platforms as `test` (`ci.yml:116-123`), under
  `if: needs.changes.outputs.editor == 'true'`. It:
  - builds `nvs-cli`
  - runs `setup-node` with `node-version-file: editors/vscode/.nvmrc` and `cache: npm`
  - runs `npm ci`, `npm run lint`, and `npm run test:headless` with `NVS_BIN` set to the built binary
  - runs `npm run package`
  - on Linux only, uploads `editors/vscode/nvs.vsix` with `actions/upload-artifact`, pinned by SHA
    like every action in the file
- **Job `extension-host`** on `ubuntu-latest`: the same setup, then `xvfb-run -a npm run test:host --
  --nvs <built binary>`, caching `editors/vscode/.vscode-test/` on the pinned VS Code version.
- Each job carries a comment block in the file's own style saying what it guards. That comment is the
  count's home, per `rule:ide/the-lockfile-is-committed-and-build-output-is-not`.

CI is not running while the billing block stands, so this stage is proven by a check that reads the two
files. Goal `gap-zero`'s green run is what proves the jobs work.

### The acceptance cache sees an extension change

`tools/loop.py:1606` fingerprints only `crates/` and `examples/` when it memoizes a check's verdict, so the
floor's cached `vsix packages` check — and any cached check over `editors/vscode` — stays green after an
extension change it never re-ran against. The fingerprint gains `editors/` (its sources, manifest and
lockfile, never `node_modules/` or build output), and a one-line test in `tools/`'s own suite pins that an
edit under `editors/vscode/src/` invalidates the memo. This goal owns it because its own host checks are
the first whose verdict would otherwise outlive the code they judged.

## Stage 7 — the five-minute fuzz run, locally

`docs/agent/commands.md:592-617`, `fuzz/seeds/prefix/`, and, only if the run finds something,
`crates/nvs-syntax/src/` and `crates/nvs-syntax/tests/prefixes.rs`.

- **The check runs the `prefix` target for 300 s under WSL** from the repository root, seeded from
  `fuzz/seeds/prefix/` as CI's job is (`ci.yml:451-458`). It passes both directories to libFuzzer rather
  than copying one into the other. M4B's *Verify* asks for exactly this: "a fuzz target over truncated
  and mid-edit inputs finds no panic in five minutes".
- **A panic it finds is a parser bug, and it is fixed**:
  - fix the site in `crates/nvs-syntax/src/`
  - add the minimized input to `fuzz/seeds/prefix/`
  - add a regression test beside `prefixes.rs`'s cuts

  The check's duration is never lowered, and the target is never narrowed.
- **`commands.md` § *Fuzzing and callgrind on Windows*** names `prefix` and the exact command. That
  section is the command's one home; the check's `argv` is its copy for the driver.

## Stage 8 — the rulebook

Three rules M4B promised are now held by something that fails when they are broken. Flip each to
`shipped` with `guardedBy` filled, then run `python tools/rules.py --render`:

- `ide/headless-gates-the-loop-the-host-run-gates-the-milestone`: `scripts/headless.mjs` and
  `test/host/index.ts`
- `ide/the-lockfile-is-committed-and-build-output-is-not`: `editors/vscode/package-lock.json` and the
  `extension` job's `npm ci`
- `ide/the-extension-runs-where-the-binary-is`: the contributions suite and the `extension` job's
  package step

## Standing decisions

- **The host tier runs on every acceptance run, and a visible window is accepted** — the user's call,
  2026-09-13. Locally it opens a VS Code window for about a minute each time; it is never skipped or
  cached to avoid that, because a stale green is the one outcome worse than the window. In CI it runs
  headless under `xvfb-run`.

- **The program's rules, set by the user for every goal from `plan-truth` to `gap-zero`.**
  - Milestones M0–M8 are complete when this chain ends, and every promise a past milestone's plan made
    is built.
  - An item may be deferred to M9+ (`docs/plan/m9.md`..`m17.md`) only if it cannot be built without
    that milestone's work, and never because it is large. M10-tagged items (format-on-save, the Test
    Explorer's coverage, PhpStorm) are valid deferrals.
  - A design call is decided under the priority ordering (ADR 0004: security > PHP-compatible
    correctness > request-path latency > simplicity > memory) and recorded. It is never `BLOCKED`.
- **CI is not running** while a GitHub billing block the user is fixing stands, so every check here runs
  locally. A workflow leg this goal adds is proven by a `command` check that reads the workflow file.
  Goal `gap-zero` needs a green CI run, and that run is what proves the legs actually work.
- **The host tier runs on this machine, on the user's call.** It is on the acceptance list and opens a
  VS Code window on the desktop while it runs.
  - It is isolated as stage 3 says: a downloaded pinned build (never the user's install), a throwaway
    profile and extensions directory, `--disable-extensions`, and a fixture copy.
  - It is **not memoized**: `tools/loop.py:1606`'s `MEMO_DIRS` hashes `crates/` and `examples/` alone,
    so a memo would stay green over an `editors/` change.
  - **Fallback**: if the pinned stable build will not start beside the user's own running stable VS
    Code, pin an Insiders build instead. Never drop the isolation flags to make it start.
- **`@vscode/test-electron`'s `runTests`, driven by our own `scripts/host.mjs`**, for two reasons: one
  launcher, and one summary line the loop greps, on both platforms. `@vscode/test-cli` is the fallback
  only if `runTests` cannot isolate the profile. Either way it is a `devDependency`, and the extension's
  runtime `dependencies` stay `vscode-languageclient` alone.
- **Colour is asserted as data, never as pixels**:
  - The TextMate layer is checked through `_workbench.captureSyntaxTokens`, the command VS Code's own
    colorize tests use. If the pinned build lacks it, pin one that has it; never weaken the assertion.
  - The semantic layer is checked through `vscode.provideDocumentSemanticTokens` and `…Legend`.
  - "Before the server answers" means the server is withheld (`nvs.lsp.enable` off), not raced.
  - *No pixel tier* stands, with its reason rewritten.
- **The test surface `activate` returns is read-only**, and exposes nothing another extension cannot
  already read. There is no reveal, no run and no setter on it; a function that changes state would
  let any installed extension unconceal a secret. It holds no language logic.
- **The `.vsix` is a build artefact, and that is already settled.** `.gitignore:98` ignores
  `/editors/*/*.vsix`, per `rule:ide/the-lockfile-is-committed-and-build-output-is-not`, so the
  `editors/vscode/nvs.vsix` on the main checkout is untracked because it is ignored. Do not re-decide
  it. CI uploads it as a workflow artifact and nothing else.
- **No binary goes into the `.vsix` or into the fixture**
  (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`). The host tier points
  `nvs.path` at the binary the build produced, in the throwaway profile only. No host test calls
  `nvs.downloadBinary` or reaches the network: the install suite's recorded fixtures are that proof, and
  they are headless.
- **The Node version has one home, `editors/vscode/.nvmrc`**, read by `setup-node` as `website/.nvmrc`
  is. The local Node here is v24.14.1.
- **WSL has the fuzz toolchain, or the session installs it.** Whether the default distro has a nightly
  toolchain and `cargo-fuzz` is *not checked*. If it does not, run `rustup toolchain install nightly`
  and `cargo install cargo-fuzz --locked` in the distro, then add both to `docs/setup.md`'s WSL list.
  The check is memoized: what it fuzzes lives under `crates/`, which the memo hashes.
- **What it spends**: nothing at run time; no byte of the binary, the server or a request changes.
  Build-side:
  - a pinned VS Code build and a throwaway profile under `editors/vscode/.vscode-test/` (ignored,
    `.gitignore:97`), once per machine
  - the growing corpus under `fuzz/corpus/prefix/` (ignored)
  - two more CI jobs, one of them a three-platform matrix
  - one host run per acceptance sweep, which is a window and about a minute
- **ADR slots**: the one record of stage 2. It creates no rule and modifies one.
- **Not this goal** — a session that finds one of these on its path writes it to the handoff's
  `## Backlog`:
  - publishing to the Marketplace or Open VSX, or attaching the `.vsix` to a release
    (`rule:ide/one-server-two-thin-clients` *Revisiting* keeps that open)
  - a pixel tier
  - PhpStorm, and every M10 editor feature
  - the stale trivia paragraph in `docs/plan/m4b.md`, which is goal `plan-truth`'s
  - the `unowned` module-doc gaps in `crates/nvs-lsp/src/index.rs` and `hints.rs`, which are goal
    `unowned-closures`'s
  - widening `MEMO_DIRS` in `tools/loop.py` to cover `editors/`
