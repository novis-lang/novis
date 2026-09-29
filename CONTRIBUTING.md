# Contributing to Novis

Novis is pre-alpha. It runs, but nothing about it is released or settled and much of the language is still
being decided, which makes this the point where an opinion is worth more than a patch — and, for now, the
point where a patch is the one thing we cannot take. **Ideas, reports and questions are open to anyone at any time; pull requests are not open
yet.** This file routes the first, explains the second, and its second half is the developer's map of the
tree.

> **Status: pre-alpha.** The language runs: `nvs run` compiles and executes a `.nvs` file, `nvs test`
> runs the conformance trees, `nvs serve` answers HTTP. Nothing is released — the version is `0.0.1`
> with no tag behind it — and nothing is stable. Work runs as a chain of numbered goals rather than as
> a march through milestones; `bun nv orient --full` prints the live one.

## Where to take what you have

| You have | Take it to |
|---|---|
| An idea, an opinion, a question, a "why is it like that?" | [Discussions](https://github.com/novis-lang/novis/discussions) |
| A bug, something broken, a doc that is wrong, a concrete proposal | [Issues](https://github.com/novis-lang/novis/issues) |
| A hole in one of the language's security claims | **neither of those** — [SECURITY.md](SECURITY.md), privately |
| The wish to hang around, follow along, or help day to day | [Discord](https://discord.gg/8ftMjPeH8h) |
| A code change | not yet — [pull requests are not open](#pull-requests-are-not-open-yet), below |

If you are not sure which of the first two it is, open a discussion. Someone will move it.

## Pull requests are not open yet

**We are not accepting pull requests from outside the core team right now — and this restriction is about
code and nothing else.** Ideas, bug reports, questions, disagreements and everything else on this page are
welcome at any time, from anyone, and always will be. Nothing below narrows that.

The restriction is a deliberate stage rather than a review backlog: Novis is being built out to one vision,
and until the core has stopped moving there is no way to judge an outside change against a design that is
still settling. A language also absorbs an inconsistent decision far more expensively than a program
does — every one of them becomes a compatibility promise the moment code runs on it.

This is temporary. Once the language is complete enough that we are happy with it and the core is settled,
pull requests open and maintenance becomes a shared job between the community and the core team. That will
be announced here, in [Discussions](https://github.com/novis-lang/novis/discussions) and on
[Discord](https://discord.gg/8ftMjPeH8h).

Until then, everything else on this page is open and none of it is a consolation prize — at this stage an
argument against a decision changes more than a patch would. Read the tree, build it, fork it, disagree
with it. The one thing that cannot be merged today is a diff.

## Helping without writing code

All of this is open now, permanently, to anyone. A language is decided long before it is compiled, and what
decides whether Novis is any good is mostly not Rust:

- **Say what you think of the design.** The type system, the `tainted`/`secret` rules, the naming of the
  `Core` library, the decision that every function is a method — all of it is written down and none of it
  is set in stone. The rulebook under [docs/rules/](docs/rules/) states every rule, and
  [docs/decisions/](docs/decisions/) holds one frozen record per decision with the reasoning behind it;
  disagreeing with one, in a discussion, is a contribution.
- **Say what would stop you using it.** A missing feature, a migration you cannot see a path through, a
  guarantee you would not trust. That is the most useful thing an outsider can report, and the hardest
  thing for the people inside to notice.
- **Report anything wrong.** A broken link, a doc that contradicts itself, an example that could not
  possibly work, a claim on the website that overstates what is true. Every fact here is supposed to have
  exactly one home; two files disagreeing is a bug, and a report of one is welcome.
- **Ask questions.** A question that is hard to answer usually means the documentation is missing, and the
  answer becomes the fix.
- **Improve the writing.** The docs, the website, the error messages: clearer wording needs no Rust and
  changes how the language is understood. Quote the sentence in an issue and say what it should say
  instead — that lands the same fix without a patch.
- **Be around.** Answering someone else's question on [Discord](https://discord.gg/8ftMjPeH8h) or in a
  discussion, or telling people the project exists, is real work and it is short-handed.

None of this needs permission, an introduction or a plan. Open the discussion.

## Working on the code

The rest of this file is the developer's half of [README.md](README.md): how the workspace is laid out,
what the compiler does with a `.nvs` file, and how to build and check the tree. Pull requests are
[not open yet](#pull-requests-are-not-open-yet), so today this is for reading the tree, building it,
forking it, and reporting what you find in it — and it is what the core team works from.

### Read this first

[AGENTS.md](AGENTS.md) carries the priority ordering every design choice is judged against, the invariants
that are easy to break, and a table pointing at the *one* document to open for a given piece of work. It is
harness-neutral: `CLAUDE.md` is a pointer to it, and `.claude/` holds Claude Code settings and nothing else.

Every fact in this repository has a single home. If two documents disagree, the one AGENTS.md names is
right and the other is a bug — fix it rather than reconciling it in your head. Two calls route you to
everything else:

```sh
bun nv orient --full           # where the work stands, one line per module, the rules
bun nv brief --where <word>    # which file owns a topic
```

### Building

Requires the pinned toolchain in [`rust-toolchain.toml`](rust-toolchain.toml), which `rustup` installs on
the first `cargo` command in the tree. Everything else a development machine needs — the MSVC build tools,
WSL and PHP 8.5 on Windows, valgrind on Linux — is [docs/setup.md](docs/setup.md).

```sh
cargo build            # debug; dependencies are still built with opt-level 2
cargo test             # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The cost baselines are release-mode figures, so their guard tests are skipped in a debug build. The
extension-sandbox probes pull in Wasmtime and sit behind a feature flag, so day-to-day builds do not pay
for it:

```sh
cargo test --release -p nvs-abi-probe                          # cost guards
cargo test --release -p nvs-abi-probe --features wasm-probe    # + sandbox probes
cargo bench -p nvs-abi-probe                                   # track the numbers
```

`bun nv verify` runs build, fmt, test, the `.nvst` trees and clippy in that order, stopping at the
first failure — one call instead of the four above.

The docs carry their own gates, which `nv verify` deliberately does not run because they need no
toolchain and finish in about a second together. They are CI's `docs` job, and a change to `docs/` or to
this file is checked by them and by nothing else:

```sh
bun nv links             # every markdown link resolves, with matching case
bun nv layout            # the layout listing below still describes the tree
bun nv records --check   # a record's field set, heading order, and the derived counters
bun nv plan --check      # every milestone row agrees with the file it names
```

[docs/agent/commands.md](docs/agent/commands.md) is the full set of tools this repo is driven by.

### Repository layout

**Every row below is on disk today**, and `bun nv layout --check` is the gate that keeps that
true: it fails if a row names something that is not there, if a crate or a tracked top-level directory
has no row, or if an `[audited unsafe]` marker disagrees with the crate's own lint policy — that marker
means the crate overrides the workspace's `unsafe_code = "forbid"` with its own audited `deny`. What the
workspace has *not* built yet — the extension loader, the formatter, the language server, the PHP
converter, the package manager — is scheduled in [docs/implementation-plan.md](docs/implementation-plan.md)
and named nowhere else, because a second copy of a schedule is how this section last went stale.

<!-- layout:begin  bun nv layout --check gates this listing; see tools/nv/cmd/layout.ts -->
```
crates/             the Cargo workspace
  nvs-diagnostics   spans, source maps, the one diagnostic record
  nvs-render        that record's three renderings: terminal, JSON, LSP
  nvs-syntax        lexer (inline HTML + PHP mode), parser, AST
  nvs-hir           name resolution, namespaces, class graph, compile-time autoload
  nvs-types         declared types, unions, narrowing, no inference
  nvs-ir            CFG/SSA IR, safepoints, refcount ops
  nvs-codegen       Cranelift backend  [audited unsafe]
  nvs-runtime       values, arrays, strings, refcounting  [audited unsafe]
  nvs-host          thread-per-core scheduler, reactor, coroutines, the Isolate boundary  [audited unsafe]
  nvs-stdlib        every `Core` member, and the registry a call resolves against  [audited unsafe]
  nvs-db            the five SQL drivers: sans-IO codecs over nvs-host's streams
  nvs-config        the directive registry and the `nvs.toml` tree  [audited unsafe]
  nvs-server        the built-in HTTP server: a socket to a root isolate and back
  nvs-cli           the `nvs` binary  [audited unsafe]
  nvs-test          the `.nvst` conformance format and its runner
  nvs-lsp           `nvs lsp` — the Novis language server
  nvs-fmt           `nvs fmt` — the Novis formatter
  nvs-repo          how a test reaches a file outside its own package, recorded (dev-only)
  nvs-footprint     what an `nvs` process resolved and read, logged for observed selection
benches/
  abi-probe         architecture invariants + cost baselines  [audited unsafe]
  serve-probe       the load generator of the server's throughput leg
  userland          the same program in Novis and in PHP, for `bun nv bench`
  members           per-`Core`-member figures, and their calibration
  serve             the request-path pair the server comparison runs
  proxied           compose files, nginx and PHP configuration for the proxied comparison
tests/              the case trees: conformance, differential, db, hostile, config
examples/           `.nvs` programs the guard tests and the docs compile
fuzz/               cargo-fuzz targets
vendor/             html5ever, patched and built in place of the crates.io release (ADR 0223)
docker/             the container image
website/            the Astro site
editors/            the VS Code extension, and where a second editor's would sit
tools/              how this repository is driven; docs/agent/commands.md is the full set
data/               the JSON records `bun nv` reads and writes: goals, gaps, rules, decisions, playbook
docs/
  setup.md          what a development machine installs, and what a clone does not carry
  novis.md          the one-file reference to everything Novis has (generated)
  implementation-plan.md   the milestone index, and the status block a session overwrites
  adr/              one decision per file, plus the topic routing table
  agent/            how agents work here: loop, goals, prompts, conventions, live handoff
  plan/             one file per milestone, plus the frozen pre-M0 design
  spec/             the normative language reference
  reference/        the chapters `bun nv reference` generates `docs/novis.md` from
  perf/             recorded benchmark figures
  examples/         worked `Core` examples
.github/            CI: three platforms, miri, asan and fuzz legs, and a Bun-only docs job
.claude/            Claude Code settings, and the pointer file to AGENTS.md
```
<!-- layout:end -->

`docs/agent/` holds how work is driven here: the [work-loop design](docs/agent/coordinator.md), the
[per-session prompt](docs/agent/session-prompt.md) and the goals' prose; a goal's record and its
handoff — live state, overwritten by each session — are under `data/goals/`. The schedule is the goal
chain, `data/chain.json`, described in [docs/agent/goals/](docs/agent/goals/README.md), not the milestone table: a milestone is an
identity tag one or more goals carry, so "goal `parses`" says what is happening and "M7" does not.

The two editor clients are not here yet. When they arrive they sit outside the Cargo workspace — the VS
Code extension is TypeScript/Node tooling, the PhpStorm plugin is Kotlin/Gradle/IntelliJ Platform
tooling — and each is a thin client over `nvs-lsp`/`nvs-fmt`, never a second implementation of language
smarts or formatting ([ADR 0016](docs/decisions/0016.md)).

[`benches/abi-probe`](benches/abi-probe/) is worth knowing about early. Several decisions in `docs/decisions/`
depend on how Cranelift, `corosensei` and Wasmtime behave rather than on Novis's own code, so a dependency
bump can invalidate them silently. It checks them continuously: that a throw propagates and a runtime
panic is *contained* across native frames, that a coroutine can suspend from beneath live JIT frames, that
a wasm guest cannot read past the host heap or outlive its deadline, that an OS process still costs orders
of magnitude more than a task — and that native unwinding through JIT frames is still unavailable, which is
the premise the calling convention exists for.

### Design in one page

| | |
|---|---|
| Pipeline | source → AST → HIR → types → IR → Cranelift → native |
| Execution | baseline JIT now, optimising tier later, no interpreter |
| Errors | checked return status, never unwinding ([ADR 0002](docs/decisions/0002.md)) |
| Concurrency | thread-per-core executors, stackful coroutines, isolated cross-core workers |
| Priorities | security → semantics → latency → simplicity → memory footprint ([ADR 0004](docs/decisions/0004.md)) |
| Types | static and mandatory; explicit checked conversions; unions plus `mixed`; `int` and `uint`; string-keyed ordered arrays with declarable nested element types ([ADR 0007](docs/decisions/0007.md)) |
| Scoping | `static` is a class-member modifier only — static members and late static binding kept, function-scope `static` and `static fn` rejected, no `global` ([ADR 0008](docs/decisions/0008.md)) |
| OOP-only | Every function is a method, every constant a class constant — no free function, no global constant, no exception for built-ins. Built-ins live under the reserved `Core` namespace, one domain class per grouping (`Core\Str`, `Core\Arr`, `Core\Math`, …) ([ADR 0011](docs/decisions/0011.md)) |
| Values | 16-byte tagged, refcounted, copy-on-write arrays and strings |
| Requests | shared-nothing; only compiled code is shared |
| Isolates | `spawn script` runs another `.nvs` file in-process with a fresh heap, on the caller's budget ([ADR 0006](docs/decisions/0006.md)) |
| Config | a tree of root-owned `nvs.toml` files states the defaults; a script may retune its own limits within operator-set ceilings, and `nvs config check` audits the result offline ([ADR 0005](docs/decisions/0005.md), [ADR 0103](docs/decisions/0103.md)) |
| Serving | built-in HTTP/1.1 with exactly two deployments — a development server, and a proxied production origin that replaces FastCGI. No TLS listener, no h2c, no compression: a proxy does each earlier and better. A filesystem path is never derived from a URL at request time ([ADR 0097](docs/decisions/0097.md)) |
| Extensions | built-in, sandboxed wasm (`.nvsx`), or statically linked native — never `dlopen` ([ADR 0003](docs/decisions/0003.md)) |

This is the short form. The fuller decision table, with the sequencing each choice implies, is in
[docs/implementation-plan.md](docs/implementation-plan.md); the reasoning behind each choice and the
measurements backing it are in the records under [docs/decisions/](docs/decisions/), each reached through a
rule's `because` in [docs/ground-rules.md](docs/ground-rules.md).

### Making a change

- [docs/agent/conventions.md](docs/agent/conventions.md) is the *shape* of what you are about to
  write — a commit message, a `.nvst` case, a `Core` member, a decision record, a diagnostic code. Read it instead of
  opening an example to copy.
- [docs/agent/playbook.md](docs/agent/playbook.md) is the trap list: the things that look like they should
  work and do not. Add a bullet when one costs you time.
- Keep each change small and commit it on its own, so `git log` reads a slice at a time.
- A decision that would be expensive to reverse earns a record under `docs/decisions/`, written by hand to
  the shape [docs/agent/conventions.md](docs/agent/conventions.md) § *A decision record* gives, and the rule
  it changes under `docs/rules/` is edited in the same slice.
- Every added, changed or removed feature owes a statement of its tradeoffs in performance, memory,
  usability and simplicity. Where they are large, raise them before building.

Novis ships under the [MIT licence](LICENSE), and contributions — once pull requests open — are accepted
under that same one.
