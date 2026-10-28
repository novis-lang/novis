---
looper: 1
title: "A broken mount, an html hole and a class file checked from anywhere fail only where they should"
summary: "nvs serve starts with the mounts that compile and names the ones that do not, a call in an html template's hole is checked as it is outside one, the agent hook finds a class file's lending program from the agent's project folder and nvs check says where it looked and takes --root, a test file that mentions require is lent its bootstrap's map, a diagnostic's path has no verbatim prefix, E0409 and E0428 name lateinit, a removed Core option's error names its replacement, the reference states config check --as and that the origin check comes before route matching, and the checks below pass."
context: [crates/nvs-hir/src/members.rs, crates/nvs-hir/src/lenders.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/agent.rs, crates/nvs-cli/src/serve.rs]
stages:
  - id: 1
    title: "a call in an html template's hole reports what it reports outside one"
    status: done
    context: [crates/nvs-hir/src/members.rs, crates/nvs-types/src/expr/calls.rs, crates/nvs-types/src/expr/literals.rs]
  - id: 2
    title: "a diagnostic names a file without the Windows verbatim prefix"
    status: done
    context: [crates/nvs-diagnostics/src/source.rs, crates/nvs-config/src/trust.rs, crates/nvs-lsp/src/grants.rs, crates/nvs-footprint/src/lib.rs]
  - id: 3
    title: "a class file and a test file are lent their program's map from wherever the check runs"
    status: done
    context: [crates/nvs-hir/src/lenders.rs, crates/nvs-cli/src/main.rs, crates/nvs-cli/src/agent.rs, docs/rules/ide/an-autoloaded-file-borrows-its-programs-map.md]
  - id: 4
    title: "E0409 and E0428 name lateinit, and a removed Core option's error names its replacement"
    status: open
    context: [crates/nvs-types/src/ctor_init.rs, crates/nvs-diagnostics/src/code/types.rs]
  - id: 5
    title: "nvs serve starts with the mounts that compile and names the ones that do not"
    status: open
    context: [crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/serve/mounts.rs, crates/nvs-cli/src/script.rs, crates/nvs-cli/src/service.rs, docs/reference/tools/25-server.md]
  - id: 6
    title: "the reference states config check --as and that the origin check comes before route matching"
    status: open
    context: [docs/reference/tools/10-cli.md, crates/nvs-stdlib/src/router.rs, docs/examples/types/Router-Refusal/about.md, docs/rules/security/csrf-is-on-by-default.md]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks: []
---

# A broken mount, an html hole and a class file checked from anywhere fail only where they should

## Why

Somebody running many modules behind one `nvs serve` found these on `56d4a7e18`. Every
reproducer below is a neutral rewrite of what they reported.

- **One mount that does not compile stops the server from starting.** `nvs serve` over a `scan`
  mount table exits 1 when any one scanned entry fails, so a binary swap that removes a `Core`
  option turns one stale module into an outage. A running server already keeps such a mount and
  fails only its requests (`crates/nvs-cli/src/serve.rs:542-547`).
- **A static call in an html template's `<?= ?>` hole is not checked, and lowering then panics.**
  `nvs-hir`'s member walk has no `ExprKind::HtmlTemplate` arm (`crates/nvs-hir/src/members.rs:1137`,
  catch-all at `:1425`), so every check only that walk makes is lost in a hole: an unknown class
  (`E0303`), a missing member, an unknown free function, a constant, `new`, `self::`. `nvs-types`
  trusts the walk, records no target, and `crates/nvs-ir/src/lower/expr.rs:4758` panics.
- **`nvs check` on a class file reports a false `E0306` per `use` when run from a subdirectory**,
  and the agent hook blocks a correct edit the same way. The lender search reads only the working
  directory (`survey_root`, `crates/nvs-cli/src/main.rs:1521`), and the hook reruns itself in the
  agent's `cwd` (`crates/nvs-cli/src/agent.rs:2672`).
- **A test file that mentions `require` is not lent its bootstrap's map.** `is_plain` is a substring
  test (`crates/nvs-hir/src/lenders.rs:200`), so `// does not require a login` in a test makes
  `nvs check tests/CartTest.nvs` report `E0306` while `nvs test` passes.
- **A diagnostic in an autoloaded file prints `\\?\E:\...`.** `SourceMap::load` shows the canonical
  path as the file's name (`crates/nvs-diagnostics/src/source.rs:419`, also `:386`).
- **Four texts say less than the behaviour.** E0409 and E0428 never name `lateinit` or `LateInit`;
  E0454 for a removed `#[Core\Route]` option lists the options left and not what replaced it; the
  `nvs config check` reference omits `--as`; the `Core\Router\Refusal` card says `NotFound` is "no
  route has this path", while a cross-origin unsafe write to such a path is `Csrf`, because the
  origin check runs first (`crates/nvs-server/src/route.rs:257-267`).

## Standing decisions

**The user decided:**
- **The agent hook searches the agent's project folder, not its `cwd`.** Claude Code gives every
  hook `CLAUDE_PROJECT_DIR`; Cursor sends `workspace_roots`. That is the tree the editor's language
  server surveys, so the hook and the editor agree. `nvs check` keeps the working directory (or the
  first `--config` file's directory) as its root. When it finds no lender and reports `E0303` or
  `E0306`, the first such diagnostic carries a note naming the folder it searched and `--root`. A new
  `nvs check --root <dir>` names the folder. An upward walk from the file was declined: in a
  workspace of modules with no marker per module, the lender sits in a sibling folder (`public/`)
  or another module, so a walk reads everything above the file on every real typo, with nothing
  to stop it.
- **`nvs serve` starts with the mounts that compile.** A mount that does not compile at start is
  kept and fails its own requests with a `500`, exactly as a running server does after an edit. Its
  diagnostic goes to the log, and `nvs service status` lists the mount. The start still fails when no
  mount compiles, so a server with nothing to serve does not restart in a loop.
- **No deprecation window for `Core`'s own API while Novis is Pre-Alpha.** A removed option stays an
  error, and its help names the replacement. A short list of removed names and their replacements
  feeds that help. The decision and its reasons become a rule fragment.

**The agent's calls:**
- **"Plain" is decided from tokens, not text.** A file is plain when no `require`, `require_once`
  or `autoload` keyword token is in its code, except one after `->`, `?->`, `::` or `function`. A
  comment, a string or a member of that name never counts. A `require` anywhere counts, not only at
  the top level, because the program's walk follows every one (`crates/nvs-hir/src/requires.rs`'s
  `walk_expr`). A text without either word is not lexed. A test file that really `require`s stays a
  program of its own, as the rule says.
- **`--root` must name a folder**, and the note names the searched folder absolute, through
  `plain_path`, so `.` is never what the reader sees.
- **The display name drops the verbatim prefix and keeps `\\?\UNC\` as a UNC path.** The stored
  path and `canonical_key` stay verbatim, so file identity is unchanged. The helper is
  `nvs_diagnostics::plain_path`, and `trust.rs` and `grants.rs` call it. `nvs-footprint` keeps its
  own `shown`: `nvs-diagnostics` depends on `nvs-footprint`, so the call would be a cycle, and
  `shown` also turns `\` into `/`, which no diagnostic wants. `trust.rs`'s canonical UNC path is now
  `\\host\share` too, where it stayed verbatim before: both name the same file.
- **A mount kept failing at start is checked for its origin when it first compiles.** Today
  `unreached_origin` runs only for rows not already standing (`crates/nvs-cli/src/serve/mounts.rs:386-402`),
  so a row kept failing would skip it. That gap fails closed (`urlAbsolute` throws), but stage 5
  closes it.
- **The test-runner page says `nvs check` needs `tests/bootstrap.nvs` too**, beside where it says
  the editor does (`website/src/content/docs/tools/test-runner/index.mdx:40`).

**Trade-offs.**
- *Serving the mounts that compile:* request isolation is unchanged, because a broken mount matches
  no route and answers `500`. The start-up promise "a program with a diagnostic never answers a
  request" becomes per mount. A deploy that waits only for the port counts a partly broken start as
  healthy, which `service status` answers. Each request to a never-compiled mount costs one read
  and one content hash of its file, on that mount only. No memory beyond the failure message already
  cached.
- *The html hole arm:* one more walk over a hole's expressions at check time, nothing at run time.
- *The lender search:* the hook reads the project folder once per check that needs a lender, as
  the editor does; `--root` and the note add no cost to a check that succeeds.
- *The display name:* one prefix test per loaded file.

**Gates.** None between stages: each touches its own files. Stage 3 edits `crates/nvs-cli/src/main.rs`
and stage 5 `crates/nvs-cli/src/serve.rs`, so they run in one tree one after the other, or as lanes.

## Stages

### 1. a call in an html template's hole reports what it reports outside one

**Does:** every expression in a `<?= ?>` or `{$ }` hole is walked by `nvs-hir`'s member walk, so
`nvs check` reports in a hole what it reports outside one, and `nvs run` never panics on it.

- [x] A `.nvst` case under `tests/conformance/` with a class method returning
  `` html`<p><?= Nope::show() ?></p>` `` expects `E0303`, and one with `self::nope()` expects the
  missing-member error. Both fail first. `tests/conformance/reject/a-secret-value-in-an-html-template-hole-is-refused.nvst:1`
- [x] `walk_expr` matches `ExprKind::Interpolated(parts) | ExprKind::HtmlTemplate(parts)`, as
  `crates/nvs-hir/src/requires.rs:1486` does. `crates/nvs-hir/src/members.rs:1137`
- [x] `Core\Str::nope()` and `Core\Str::length(5)` in a hole report what they report outside one.
  If either still checks clean, find where `nvs-types` loses it. `crates/nvs-types/src/expr/calls.rs:505`
- [x] A case pins an unknown free function and `new Nope()` in a hole too.

### 2. a diagnostic names a file without the Windows verbatim prefix

**Does:** a diagnostic in a file reached through `autoload` or `require` names it as `E:\...`, never
`\\?\E:\...`.

- [x] The prefix-stripping helper lives in `nvs-diagnostics`, keeping `\\?\UNC\` as `\\`, with unit
  cases for a drive path, a UNC path and a non-Windows no-op. `crates/nvs-config/src/trust.rs:674`
- [x] `SourceMap::load` and the overlay branch name the file with it. `crates/nvs-diagnostics/src/source.rs:386`, `:419`
- [x] `trust.rs` and `crates/nvs-lsp/src/grants.rs:169` call it; `crates/nvs-footprint/src/lib.rs:176`
  cannot (see the standing decision).
- [x] A Windows-only CLI test checks a class file through autoload and finds no `\\?\` in the output.

### 3. a class file and a test file are lent their program's map from wherever the check runs

**Does:** the hook finds the lender from the agent's project folder, `nvs check` says where it looked
and takes `--root`, and a test file that mentions `require` in a comment is still plain.

- [x] `is_plain` reads top-level `require` and `autoload` statements from tokens, with cases for
  a comment, a string and a real statement. `crates/nvs-hir/src/lenders.rs:200`, `crates/nvs-cli/src/main.rs:1169`
- [x] `nvs check --root <dir>` sets the survey root, and when no lender is found the first
  `E0303`/`E0306` carries a note naming the folder searched. `crates/nvs-cli/src/main.rs:1521`
- [x] The hook surveys `CLAUDE_PROJECT_DIR`, else `workspace_roots[0]`, else `cwd`, with a test
  whose `cwd` is a subdirectory of the project. `crates/nvs-cli/src/agent.rs:2665-2700`
- [x] The rule fragment states the hook's root, `--root` and the note, and its Declined list keeps
  the upward walk with the reason above. `docs/rules/ide/an-autoloaded-file-borrows-its-programs-map.md`
- [x] The test-runner page says `nvs check` needs `tests/bootstrap.nvs` too, and the CLI reference
  documents `--root`. `website/src/content/docs/tools/test-runner/index.mdx:40`, `docs/reference/tools/10-cli.md`

### 4. E0409 and E0428 name lateinit, and a removed Core option's error names its replacement

**Does:** the help of E0409 and E0428 offers `lateinit` and `LateInit`, and E0454 for a removed
`#[Core\Route]` option says what replaced it.

- [ ] E0409's help and card add: a property whose value needs `$this` can be `lateinit`, and a class
  that implements `LateInit` sets it on the first read. `crates/nvs-types/src/ctor_init.rs:514`, `crates/nvs-diagnostics/src/code/types.rs:69`
- [ ] E0428's card and help add `LateInit` the same way. `crates/nvs-diagnostics/src/code/types.rs:213`
- [ ] A list of removed `Core` attribute options and their replacements feeds E0454's help, starting
  with `#[Core\Route]`'s `name` → `Core\Router::url(Home::show)`, with a `.nvst` case.
- [ ] A rule fragment states that `Core`'s own API has no deprecation window while Novis is
  Pre-Alpha, and that a removed name's error names its replacement, with its reasons and Declined list.

Every help line and card follows AGENTS.md § *Text an end user reads*.

### 5. nvs serve starts with the mounts that compile and names the ones that do not

**Does:** a mount that does not compile at start fails only its own requests, the log and
`nvs service status` name it, and the start fails only when no mount compiles.

- [ ] A CLI test starts a server over a `scan` table with one good and one broken module, and the
  good one answers while the broken one answers `500`. It fails first. `crates/nvs-cli/tests/live_edit.rs:1200`
- [ ] The start-up loop keeps a failing mount and fails only when none compiles. `crates/nvs-cli/src/serve.rs:542-547`
- [ ] A kept failing row runs `unreached_origin` when it first compiles. `crates/nvs-cli/src/serve/mounts.rs:386-402`
- [ ] `nvs service status` lists each mount that failed to compile at start. `crates/nvs-cli/src/service.rs:4714`
- [ ] The rules and reference say it: `docs/rules/http-server/the-accept-fan-out-is-one-worker-per-core.md:5`,
  `docs/rules/http-server/a-mount-table-expands-at-boot.md`, `docs/rules/http-server/health-path-is-off-and-checks-nothing.md:3`,
  `docs/reference/tools/25-server.md:14-16` and its mounts section at `:87`.

### 6. the reference states config check --as and that the origin check comes before route matching

**Does:** two reference texts match the behaviour.

- [ ] `nvs config check and nvs config dump` documents `--as <account>`. `docs/reference/tools/10-cli.md:617`
- [ ] The `Core\Router\Refusal` card says a cross-origin unsafe write is `Csrf` before any route is
  matched, so `NotFound` is a same-origin or safe request. `crates/nvs-stdlib/src/router.rs:951-959`,
  `docs/examples/types/Router-Refusal/about.md:8`
- [ ] The route-table section of the routing reference says the origin check comes first.
  `docs/rules/security/csrf-is-on-by-default.md:6`

### 7. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1, 2 and 3 are done, each new test run once by name and green. Stage 3: `is_plain` reads
tokens, `crates/nvs-hir/src/lenders.rs:208`. `nvs check --root` sets `survey_root`
(`crates/nvs-cli/src/main.rs:1547`), and `searched_note` (`:1354`) goes on the first
`E0303`/`E0306` when no lender is found. The hook's root is `project_dir`,
`crates/nvs-cli/src/agent.rs:2730`. `crates/nvs-cli/tests/agent.rs` now clears `CLAUDE_PROJECT_DIR`,
which the agent running the suite sets.

Tests that may go red in stage 7, none searched for:
- one comparing the whole stderr of an `nvs check` or a hook report whose `E0303`/`E0306` found no
  lender: it now has one more note line;
- on Windows, one comparing a diagnostic's file name with `fs::canonicalize` output (stage 2);
- one whose html template hole holds a call `nvs-hir` now reports (stage 1).

The sweep's `env-secret-proofs` log shows exit 0 on its last run. The nightly run `40662443492` was
a cancelled manual dispatch, and the push run after it, `40664047249`, is green. Neither needs a fix.

### Next

Stage 4, the lateinit help and the removed-option help, on `crates/nvs-types/src/ctor_init.rs` and
`crates/nvs-diagnostics/src/code/types.rs`. No gate. Every help line and card is end-user text.
- [ ] E0409's help and card name `lateinit` and `LateInit`. `crates/nvs-types/src/ctor_init.rs:514`, `crates/nvs-diagnostics/src/code/types.rs:69`
- [ ] E0428's help and card name `LateInit`. `crates/nvs-diagnostics/src/code/types.rs:213`
- [ ] E0454's help from the removed-option list, and a `.nvst` case. `crates/nvs-diagnostics/src/code/types.rs:465`
- [ ] The no-deprecation-window rule fragment, beside `docs/rules/core-api/reference-card.md:1`

### Backlog

- Stage 6 touches no file another stage touches and can run as a lane beside stage 4. It edits
  `docs/reference/tools/10-cli.md`, which stage 3 changed in its `nvs check` section only.
- Stage 5 edits `crates/nvs-cli/src/serve.rs` and runs after stage 4 or as its own lane.
