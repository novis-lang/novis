---
looper: 1
title: "Everything a program can call has examples, and nothing internal is on the website"
summary: "No feature on the website is excused from its examples, and a built-in component's engine class is internal to its component."
context: []
stages:
  - id: 1
    title: "`Image\\Diff`'s constructor has its examples"
    status: done
    context: [extensions/image/nvs/Diff.nvs, data/proofs/policy.json]
    extra:
      context:
        rules: [testing/feature-proofs]
        shapes: ["An example"]
  - id: 2
    title: "a built-in component's engine class is internal to its component"
    status: done
    context: [crates/nvs-types/src/expr/calls.rs, crates/nvs-ext/src/load.rs, crates/nvs-cli/src/extensions.rs, crates/nvs-diagnostics/src/lib.rs]
    extra:
      context:
        rules: [packaging/the-first-party-components-are-built-in]
        shapes: ["A `.nvst` test case", A diagnostic, A rule fragment]
  - id: 3
    title: "the engine classes leave the roster, the proofs policy and the website"
    status: done
    context: [tools/nv/proofs/roster.ts, tools/nv/renderers/website-core.ts, crates/nvs-cli/src/meta.rs, data/proofs/policy.json]
    extra:
      context:
        rules: [testing/feature-proofs]
        shapes: []
  - id: 4
    title: "a CLI command's example is a command and its checked output"
    status: open
    context: [docs/examples/README.md, tools/nv/proofs/collect.ts, website/src/lib/feature.ts, website/src/components/FeatureExamples.astro]
    extra:
      context:
        rules: [testing/feature-proofs]
        shapes: ["An example"]
  - id: 5
    title: "an example may require the Postgres service"
    status: done
    context: [docs/examples/README.md, tests/db/compose.yaml, tools/nv/proofs/collect.ts]
    extra:
      context:
        rules: [testing/feature-proofs]
        shapes: ["An example"]
checks:
  - id: diff-examples
    stage: 1
    name: "`Novis\\Image\\Diff::constructor` owes nothing, with no examples skip"
    run: [bun, nv, proofs, --gate, --only, "Novis\\Image\\Diff::constructor"]
  - id: engine-call-does-not-compile
    stage: 2
    name: "a program that calls `Novis\\Image\\Codec::info` does not compile, and the error names `Novis\\Image\\Image`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/novis/a-program-cannot-call-a-components-engine.nvst]
    tests: [tests/conformance/novis/a-program-cannot-call-a-components-engine.nvst]
  - id: engine-internal-diff
    stage: 2
    name: "the conformance cases that called an engine directly pass on the public classes"
    run: [bun, nv, suite, tests/conformance/]
  - id: engine-off-roster
    stage: 3
    name: "no roster feature is a member of a built-in component's engine class"
    run: [bun, test, tools/nv/test/roster-novis.test.ts, -t, "a built-in component's engine class is no feature"]
  - id: reference-pages
    stage: 3
    name: "every Reference page shows every example of every feature on it"
    run: [bun, nv, site, --check, reference]
  - id: cli-examples
    stage: 4
    name: "`nvs info` and `nvs agent init` owe nothing, with no examples skip"
    run: [bun, nv, proofs, --gate, --only, "tools:cli/nvs-info", "tools:agents/nvs-agent-init"]
  - id: postgres-example
    stage: 5
    name: "`Core\\Db\\Row::instant` owes nothing, with no examples skip"
    run: [bun, nv, proofs, --gate, --only, "Core\\Db\\Row::instant"]
---

# Everything a program can call has examples, and nothing internal is on the website

## Why

The user's rule: **if an end user can call or use it, it has examples; if it is internal to Novis, it
is not on the website.** The Reference shows every roster feature, and `data/proofs/policy.json`
excuses 27 of them from examples. Four groups break the rule:

- The engine classes `Novis\Image\Codec`, `Novis\Intl\Icu`, `Novis\Pdf\Engine` and
  `Novis\Spreadsheet\Engine` are a built-in component's manifest class. Every one of their 35
  methods is called by the component's own Novis classes, so they are plumbing. A program can still
  call them today (`docs/rules/packaging.md:1625`), so they are on the website without examples.
- `new Novis\Image\Diff(...)` is a public constructor with no examples.
- `Core\Db\Row::instant` has none, because every example runs on SQLite, which has no zone-carrying
  column.
- `nvs info` and `nvs agent init` have none, because an example is a `.nvs` program, and neither
  command reads one.

## Standing decisions

**The user decided each group.** The engines become internal, the CLI and Postgres cases get new
example kinds, and `Diff`'s constructor stays public and gets three examples.

**An engine class is internal to its component.** A call to a built-in component's manifest class
from any file outside that component's Novis source half is a compile error. The error names the
public class a program uses instead. The engine stays in the binary and in `nvs meta --json`, where
it carries `"internal": true`, and it leaves the proofs roster, `data/proofs/policy.json` and the
website. Every engine method is called by its component's Novis classes, so no capability is lost.
The 26 conformance cases that call an engine directly are rewritten onto the public classes. Home:
a new fragment under `docs/rules/packaging/`, beside
`rule:packaging/the-first-party-components-are-built-in`.

**A CLI example is a command and its checked output.** A tool feature's example may be
`NN-title.cmd` holding one `nvs …` command line, with `NN-title.out` beside it. The runner runs it
in a scratch directory and compares standard output byte for byte, the way it does a program. The
website shows it as a shell block. Home: `docs/examples/README.md`.

**`// requires: postgres`.** It works as `// requires: samba-ad` does: before the first such
program in a run, the runner brings up the `postgres` service of `tests/db/compose.yaml`, once.
The website shows the example as usual. Home: `docs/examples/README.md`.

## Stages

### 1. `Image\Diff`'s constructor has its examples

**Does:** `new Novis\Image\Diff(...)` has three examples, and the policy no longer excuses it.

- [x] Three examples of the constructor (`docs/examples/novis/Image-Diff/constructor/01-a-diff-from-its-values.nvs:1`, `extensions/image/nvs/Diff.nvs:9`)
- [x] Its examples skip deleted (`data/proofs/policy.json:71`)

### 2. a built-in component's engine class is internal to its component

**Does:** a call to a built-in component's manifest class from outside that component's Novis source
does not compile, and the error names the public class to use.

- [x] The check where a static call resolves its class (`crates/nvs-types/src/expr/calls.rs:359`)
- [x] Which classes are a component's engine, from its manifest (`crates/nvs-types/src/ext_lib.rs:204`)
- [x] The diagnostic and its card (`crates/nvs-diagnostics/src/lib.rs:5516`)
- [x] The new case (`tests/conformance/novis/a-program-cannot-call-a-components-engine.nvst:1`)
- [x] The cases that call an engine directly, rewritten onto the public classes (`tests/conformance/novis/intl-each-job-class-answers-for-its-export.nvst:1`)
- [x] The new fragment beside `rule:packaging/the-first-party-components-are-built-in` (`docs/rules/packaging/a-components-engine-class-is-internal.md:1`)

### 3. the engine classes leave the roster, the proofs policy and the website

**Does:** an engine class is `"internal": true` in `nvs meta --json`, and no roster feature, policy
entry or Reference page names one.

- [x] `"internal": true` on an engine class (`crates/nvs-cli/src/meta.rs:198`)
- [x] The roster skips an internal class, with its test (`tools/nv/proofs/roster.ts:1`, `tools/nv/test/roster-novis.test.ts:6`)
- [x] The engine entries deleted from the policy (`data/proofs/policy.json:41`)
- [x] The website skips an internal class (`tools/nv/renderers/website-core.ts:92`)

### 4. a CLI command's example is a command and its checked output

**Does:** `nvs info` and `nvs agent init` each have examples that are a command line and its output.

- [x] The `NN-title.cmd` kind and how it is run (`docs/examples/README.md:199`, `tools/nv/proofs/run.ts:205`)
- [x] The website shows one as a shell block (`website/src/lib/feature.ts:156`, `website/src/components/FeatureExamples.astro:28`)
- [ ] The two commands' examples, and their skips deleted (`data/proofs/policy.json:75`)
  - [x] `nvs agent init`'s three examples, and its skip deleted (`docs/examples/tools/agents/nvs-agent-init/01-set-up-a-project-for-codex.cmd:1`)
  - [ ] `nvs info`'s examples, once the user decides how a per-machine output is checked (`data/proofs/policy.json:78`)

### 5. an example may require the Postgres service

**Does:** an example marked `// requires: postgres` runs against the `postgres` service, and
`Core\Db\Row::instant` has its examples.

- [x] The marker, beside `requires: samba-ad` (`tools/nv/proofs/run.ts:77`, `docs/examples/README.md:243`, `tests/db/compose.yaml:41`)
- [x] `Core\Db\Row::instant`'s examples, and its skip deleted (`data/proofs/policy.json:18`)

## Handoff

### State

Stages 1 to 3 are done. Each built-in component's engine class, and each of its enums that no
public signature reaches, is `"internal": true` in `nvs meta --json`
(`rule:packaging/a-components-engine-class-is-internal`). They are not on the roster, in the policy
or on the website.

Stage 4 is done except `nvs info`. The `.cmd` example kind is on disk. `spawnProof` runs the line
in a scratch directory under `.agent-tmp/proof-cmd/`, and every lister of examples reads `.cmd` through
`programsIn`. The website shows the line as an `sh` block. `nvs agent init` has three examples and
no skip. `engine-off-roster` now runs its test with `-t`, because bun prints no test names under
the driver's environment.

Stage 5 is done. The generic `// requires: <service>` marker already brought up `postgres`, so the
stage needed no runner change. `Core\Db\Row::instant` has three examples on PostgreSQL, with an
`nvs.toml` whose `tls_ca_file` points at `tests/db/ca.crt`, and its examples skip is deleted.
`postgres-example` is green.

**Blocked on the user:** `nvs info` prints the commit, the build profile, the executable path,
the CPU count and one line per dependency. No `.out` matches it on two machines, so the decision's
"byte for byte" cannot hold for it. Recommendation: in a `.cmd` example's `.out`, a line that is
exactly `...` matches any number of lines. Then `nvs info`'s examples show the section headings and
the `Licensing` lines, and elide the rest. Re-bless them each release, because the `Novis 0.0.1`
line changes. The alternative is to keep its skip, with a reason that says the output differs per
machine. Then the goal's reach condition excuses it.

### Next

Stage 4's last item, once the user decides, over the runner and `nvs info`'s directory:

- The elision line in a `.cmd`'s `.out`, if chosen (`tools/nv/proofs/run.ts:399`,
  `docs/examples/README.md:199`) (`rule:testing/feature-proofs`).
- `nvs info`'s three examples, and its skip deleted (`docs/examples/tools/cli/nvs-info/about.md:1`,
  `data/proofs/policy.json:78`).
- Then the goal is reached: `cli-examples` is its last red check.

### Backlog

- `Core\Router\Gate` owes its examples, tests and attack as an interface feature (`bun nv proofs --id 'Core\Router\Gate'`).
- `bun nv side --prepare` installs neither `editors/vscode`'s nor `website`'s packages, so a side
  worktree's verify and `bun nv site --build` fail until `npm ci` runs there (`tools/nv/cmd/side.ts:90`).
