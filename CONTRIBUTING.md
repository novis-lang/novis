# Contributing to Novis

Novis is pre-alpha. Nothing runs yet and most of the language is still being decided, which makes this the
point where an opinion is worth more than a patch — and, for now, the point where a patch is the one thing
we cannot take. **Ideas, reports and questions are open to anyone at any time; pull requests are not open
yet.** This file routes the first, explains the second, and its second half is the developer's map of the
tree.

> **Status: pre-alpha, milestone M0.** The language does not run yet — `Hello World` is M3. Nothing is
> stable, and the crates listed further down are mostly the shape the workspace grows into rather than
> code you can read today.

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
  is set in stone. [docs/adr/](docs/adr/README.md) is one file per decision, each with the reasoning behind
  it; disagreeing with one, in a discussion, is a contribution.
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
python tools/brief.py                  # the plan's status, one line per module, the guard tests
python tools/brief.py --where <word>   # which file owns a topic
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

`python tools/verify.py` runs build, fmt, test, the `.nvst` trees and clippy in that order, stopping at the
first failure — one call instead of the four above.
[docs/agent/commands.md](docs/agent/commands.md) is the full set of tools this repo is driven by.

### Repository layout

Four of these exist today. The rest are the shape the workspace grows into; the right-hand column is
the milestone that creates each one, since a crate is added when its milestone starts rather than
sitting empty. `nvs-cli` is the one exception to "created when its milestone starts": M1's own plan
names `nvs ast` as its verification tool, so the crate was scaffolded early with just that one
subcommand — `run`/`test` and the rest of the CLI still arrive at M3.

```
crates/
  nvs-diagnostics   spans, source maps, error rendering                            exists
  nvs-syntax        lexer (inline HTML + PHP mode), parser, AST                        M1
  nvs-hir           name resolution, namespaces, class graph                           M2
  nvs-types         declared types, unions, narrowing, no inference                    M2
  nvs-ir            CFG/SSA IR, safepoints, refcount ops                               M2
  nvs-codegen       Cranelift backend  [audited unsafe]                                M3
  nvs-runtime       values, arrays, coroutines, scheduler  [audited unsafe]            M3
  nvs-cli           the `nvs` binary (`ast`, `check`, `run`, `test`, `info`)       exists
  nvs-stdlib        Core domain classes, native builtin static methods                 M4S
  nvs-test          .nvst runner                                                    exists
  nvs-host          Transport trait, unit cache, the Isolate boundary                  M5
  nvs-config        nvs.toml registry, changeability classes, overlays                  M6
  nvs-cache         content-addressed artifact cache                                   M6
  nvs-http          hyper h1 + h2c transport, optional rustls                          M7
  nvs-regex         two-tier engine + `preg_*` layer                                   M8
  nvs-db            driver trait + mysql / pgsql / sqlite / mssql                      M8
  nvs-ext           .nvsx loader, WIT host, per-request instancing                     M9
  nvs-fmt           formatter                                                         M10
  nvs-lsp           tower-lsp language server                                         M10
  nvs-dap           debug adapter                                                     M10
  nvs-pkg           package manager                                                   M10
  nvs-convert       PHP→Novis transpiler, .phpt→.nvst                                   M11
  nvs-fcgi          optional FastCGI transport                                        M13
editors/
  vscode            TextMate grammar, language-configuration.json, LSP client        M10
  phpstorm          file-type registration, LSP-bridge plugin (Kotlin/Gradle)         M10
benches/
  abi-probe         architecture invariants + cost baselines  [audited unsafe]     exists
tools/
  brief.py          the one-call orientation digest every session starts with      exists
  loop.py           the unattended work-loop driver                                exists
  splice.py         exact-block file edit, for edits a shell would mangle          exists
  gaps.py           the cases Part I's corpus does not ask for yet, as a worklist  exists
  check-links.py    advisory: markdown links that break, or that mis-match case    exists
  gen-attribution   generates THIRD-PARTY-LICENSES.txt from the dep graph          exists
  leak-check.sh     valgrind over named .nvs fixtures (WSL/Linux)                  exists
docs/setup.md       what a machine installs, and what a clone does not carry      exists
docs/adr/           architecture decision records + the topic routing table        exists
docs/agent/         how agents work in this repo: loop, prompts, live handoff      exists
docs/spec/          normative language reference                                unwritten
```

`docs/agent/` holds how work is driven here: the [work-loop design](docs/agent/coordinator.md), the
[per-session prompt](docs/agent/session-prompt.md), the [current goal](docs/agent/loop-goal.md), and
[handoff.md](docs/agent/handoff.md) — live state, overwritten by each session.

`editors/` sits outside the Cargo workspace — the VS Code extension is TypeScript/Node tooling, the
PhpStorm plugin is Kotlin/Gradle/IntelliJ Platform tooling — and is a thin client over `nvs-lsp`/`nvs-fmt`
in both cases, never a second implementation of language smarts or formatting
([ADR 0016](docs/adr/0016-ide-integration.md)).

[`benches/abi-probe`](benches/abi-probe/) is worth knowing about early. Several decisions in `docs/adr/`
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
| Errors | checked return status, never unwinding ([ADR 0002](docs/adr/0002-error-propagation.md)) |
| Concurrency | thread-per-core executors, stackful coroutines, isolated cross-core workers |
| Priorities | security → semantics → latency → simplicity → memory footprint ([ADR 0004](docs/adr/0004-memory-for-simplicity.md)) |
| Types | static and mandatory; explicit checked conversions; unions plus `mixed`; `int` and `uint`; string-keyed ordered arrays with declarable nested element types ([ADR 0007](docs/adr/0007-explicit-type-system.md)) |
| Scoping | `static` is a class-member modifier only — static members and late static binding kept, function-scope `static` and `static fn` rejected, no `global` ([ADR 0008](docs/adr/0008-static-and-global.md)) |
| OOP-only | Every function is a method, every constant a class constant — no free function, no global constant, no exception for built-ins. Built-ins live under the reserved `Core` namespace, one domain class per grouping (`Core\Str`, `Core\Arr`, `Core\Math`, …) ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)) |
| Values | 16-byte tagged, refcounted, copy-on-write arrays and strings |
| Requests | shared-nothing; only compiled code is shared |
| Isolates | `spawn script` runs another `.nvs` file in-process with a fresh heap, on the caller's budget ([ADR 0006](docs/adr/0006-isolated-script-execution.md)) |
| Config | root-owned `nvs.toml` states defaults; a script may retune its own limits within operator-set ceilings ([ADR 0005](docs/adr/0005-config-changeability.md)) |
| Serving | built-in HTTP/1.1 + h2c; FastCGI optional and later |
| Extensions | built-in, sandboxed wasm (`.nvsx`), or statically linked native — never `dlopen` ([ADR 0003](docs/adr/0003-extension-system.md)) |

This is the short form. The fuller decision table, with the sequencing each choice implies, is in
[docs/implementation-plan.md](docs/implementation-plan.md); the reasoning behind each choice and the
measurements backing it are in [docs/adr/](docs/adr/README.md).

### Making a change

- [docs/agent/conventions.md](docs/agent/conventions.md) is the *shape* of what you are about to
  write — a commit message, a `.nvst` case, a `Core` member, an ADR, a diagnostic code. Read it instead of
  opening an example to copy.
- [docs/agent/playbook.md](docs/agent/playbook.md) is the trap list: the things that look like they should
  work and do not. Add a bullet when one costs you time.
- Keep each change small and commit it on its own, so `git log` reads a slice at a time.
- A decision that would be expensive to reverse earns an ADR — `python tools/adr.py --draft` prints the
  skeleton, and [docs/adr/README.md](docs/adr/README.md) § *Adding a decision* is the rest of the form.
- Every added, changed or removed feature owes a statement of its tradeoffs in performance, memory,
  usability and simplicity. Where they are large, raise them before building.

Novis ships under the [MIT licence](LICENSE), and contributions — once pull requests open — are accepted
under that same one.
