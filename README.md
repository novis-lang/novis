# Novis — The Web-Native Programming Language

A programming language for the web. Secure by design, not by discipline. Untrusted data: tracked, and
blocked from anywhere dangerous. Secrets: can't leak into logs or screens, not even by accident.
Downloaded code: does only what you allow. Fast, like you would expect. Easy to write, easy to scale.
For web servers and the command line. Familiar syntax (Hello PHP).

> **Status: pre-alpha, milestone M0.** The architecture is validated by working spikes but the language
> does not run yet. `Hello World` is milestone M3. Nothing here is stable.

## What it is trying to be

- **Injection and secret leakage are compile errors.** Untrusted input carries a `tainted` type from the
  moment it enters, credentials carry `secret`, and every sink that could leak either one refuses them — so
  the whole class fails the build instead of a scan afterwards
  ([why, and how the other web languages compare](docs/why-tainted-and-secret.md)).
- **No build step.** Edit a `.nvs` file and run it. Compilation happens on load and is cached.
- **Actually compiled.** Cranelift emits native code. There is no interpreter tier.
- **Parallel in-language.** `async`/`await` for overlapping I/O plus isolated workers for real multicore
  CPU work — with no `async` function colouring, so any function may do I/O.
- **One process, many requests.** A single server process handles unlimited concurrent requests, each
  fully isolated, all sharing one in-memory compiled-code cache.
- **Isolation you can reach from the language.** `spawn script 'job.nvs'` runs another file with its own
  heap, its own globals and its own slice of the caller's budget, and starts in microseconds
  ([ADR 0006](docs/decisions/0006.md)).
- **Typed on purpose.** Every parameter, property and variable declares its type, and no value changes
  type behind your back: conversions are explicit and throw rather than quietly yielding `0`. Unions and
  `mixed` are there for the cases that genuinely are dynamic. `uint` gives you the whole unsigned 64-bit
  range, and arrays are ordered hashes with declarable, nestable element types — `array<array<uint>>`
  ([ADR 0007](docs/decisions/0007.md)).
- **Memory-safe and contained.** Written in Rust with `unsafe` confined to three audited modules. A
  runtime bug or a resource-limit breach kills one request, never the process.
- **Fast and simple first; memory is what pays for that.** Novis targets server-class hardware, so where a
  design can be safer, faster or simpler by holding more memory, it holds more memory — deliberately, within
  an enforced per-request cap ([ADR 0004](docs/decisions/0004.md)). It is not a
  low-footprint runtime, and sizing it means sizing for concurrency.
- **Extensible without giving up any of that.** Extensions are sandboxed WebAssembly components: one
  precompiled binary runs on every platform, written in whatever language you like, and a crashing or
  hostile extension harms one request rather than the process.

Coming from PHP? [How Novis is different, and how to port a program](docs/reference/tools/30-php-differences.md).

## Roadmap

The full plan — milestones broken into deliverables, with their verification criteria and the reasoning
behind each design decision — is [docs/implementation-plan.md](docs/implementation-plan.md).

| | Milestone | State |
|---|---|---|
| M0 | Project setup, CI, architecture spikes | **done** |
| M1 | Lexer, parser, diagnostics | **next** |
| M2 | HIR, type system, IR | |
| M3 | Baseline Cranelift backend → **Hello World** | |
| M4 | Language completeness, test runner | |
| M5 | Concurrency: coroutines, channels, workers, script isolates | |
| M6 | `nvs.toml`, capabilities, limits, artifact cache | |
| M7 | Built-in HTTP server | |
| M8 | Stdlib, database drivers, the `nvs:ext` WIT world | |
| M9 | Extension system: `.nvsx` loading, sandboxing, `nvs ext` tooling | |
| M10 | LSP, formatter, debugger, profiler, package manager | |
| M12 | Optimising JIT tier | |
| M13 | Optional FastCGI transport | if a deployment target needs it |

## Contributing

Novis is early enough that an opinion is worth more than a patch. Ideas, questions and disagreements go to
[Discussions](https://github.com/novis-lang/novis/discussions), bugs and concrete proposals to
[Issues](https://github.com/novis-lang/novis/issues), and [Discord](https://discord.gg/8ftMjPeH8h) is
where the day-to-day conversation happens — all of that is open to anyone, at any time.

**Pull requests are the exception: they are not open yet.** The core team is building the language out to
one vision first, and they open once it is settled, so that maintenance becomes a shared job between the
community and the core team. Code contributions are the only thing this affects.

[CONTRIBUTING.md](CONTRIBUTING.md) routes all of that, and its second half is the developer's half of this
file: the design in one page, the repository layout, how to build and check the tree, and the conventions a
change is expected to follow. Start there, then [AGENTS.md](AGENTS.md) — it carries the priority ordering
every design choice is judged against, the invariants that are easy to break, and a table pointing at the
*one* document to open for a given piece of work.

## Sponsoring

Novis is open source and free, and it always will be. What it costs is time — and time is exactly what we
are willing to invest to reach our goal. But some things already cost real money: servers, licences,
hardware. Novis has a baseline that is paid every month.

That is where sponsors come in — maybe you?! Sponsors help us cover those fixed costs. Right now we are not
a company, just individuals trying to build something great, and today a donation is received by a private
individual: Roland, the creator of Novis. The plan is already set out, though: when the time comes we will
found a *gemeinnütziger Verein* in Austria — a charitable association, the best legal form for this
project. It is a non-profit whose purpose is not to make money, and it is what lets us be a proper
organisation with transparent financial reports, employ people, rent an office, and keep the machine
running. Before founding anything, though, the essentials come first — and sponsored money will be
published transparently through the [Open Source Collective](https://opencollective.com/).

Every donation helps, no matter how large, how small, how long or how short. Ask on
[Discord](https://discord.gg/8ftMjPeH8h) and we will tell you how — the full story is on the website's
sponsoring page.

And if you would rather not donate, or cannot: come to us with ideas and discussion instead, any time.
That is what [CONTRIBUTING.md](CONTRIBUTING.md) is for.

## Licence

Novis is [MIT](LICENSE). Contributions are accepted under the same licence.

Every third-party component compiled into the `nvs` binary is permissively licensed and attributed in
[THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt), with each component's own copyright notice and
licence text reproduced in full. That file is **generated** from the resolved dependency graph and
**embedded in the binary**, so a copy of `nvs` carries its notices without the repository:

```sh
nvs info                # build and host facts, plus every component and its licence
nvs info --licenses     # the same, plus every licence text in full

bun nv gen-attribution           # regenerate after changing a dependency
bun nv gen-attribution --check   # what CI runs; fails if the notice is stale
```

The generator fails closed: a licence it has no policy for, or one missing from
[deny.toml](deny.toml)'s allow list, stops the build instead of quietly omitting a notice. `cargo deny`
decides what may be *linked*; this decides what must be *shipped*
([ADR 0065](docs/decisions/0065.md)).
