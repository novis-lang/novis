# Changelog

Every released version of Novis, newest first. Generated from the commit log by
`bun nv release` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is
[`rule:packaging/the-versioned-surface-is-enumerated`](docs/rules/packaging.md#packaging-the-versioned-surface-is-enumerated):
it covers the language, the `Core` library, `nvs.toml`, the CLI, diagnostic identity, the
extension ABI and `serialize()` output -- and explicitly not the Rust APIs of the `nvs-*` crates.
Before 1.0 the breaking slot moves left: `0.MINOR` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.

## [0.0.2] - 2026-11-03

### Features

- **lsp**: a body-only edit of an open file re-queues no reader ([`84172184`](https://github.com/novis-lang/novis/commit/84172184433eeea9ba51f4fac6495dd294a4dda4))
- **lsp**: a file is parsed again only when its text changed ([`207a0cd4`](https://github.com/novis-lang/novis/commit/207a0cd4674b468c6ce2b814c176108e9db15a4a))
- **lsp**: the editor checks only the bodies whose inputs changed, against one session interner ([`4e0674e8`](https://github.com/novis-lang/novis/commit/4e0674e8cce327d17c3459ba314885c6e3f1f4b4))
- **lsp**: the symbol index is kept on disk, and a start analyses only its stale entries ([`59eb4b7f`](https://github.com/novis-lang/novis/commit/59eb4b7f4cd60f171137a0d99c0be431644039c2))
- **site**: the website counts page views with a cookieless Matomo on stats.novis-lang.org ([`da19c63e`](https://github.com/novis-lang/novis/commit/da19c63e1091ffe261a879a791b9905ee189af8a))
- **tools**: NVS_PROOF_TIMES writes each proof program's wall time, slowest first ([`f62d0639`](https://github.com/novis-lang/novis/commit/f62d063984e5a5c7e32209606169a6b65c2581d3))
- **types**: a check walks the bodies of a chosen set of files and folds every other file's cached output ([`297f9356`](https://github.com/novis-lang/novis/commit/297f935629565bc0a81299be015467c5845f5cf2))
- **vscode**: the nvs.path setting links the download and the release page ([`1e6737b7`](https://github.com/novis-lang/novis/commit/1e6737b7c6240011c979b7b0403061f194176421))

### Fixes

- **cli**: nvs run's stop cancels a queue worker still opening its connection ([`63a6861d`](https://github.com/novis-lang/novis/commit/63a6861d44bb8588bf152a90baf48dc67468f310))
- **cli**: nvs run's stop cancels a queue worker still opening its connection again ([`45e4ac4c`](https://github.com/novis-lang/novis/commit/45e4ac4c7c3db5e521e098171b3359f877455f2a))
- **cli**: nvs run's stop no longer cancels a queue worker still opening its connection ([`cb931f1e`](https://github.com/novis-lang/novis/commit/cb931f1efaded760478f1e2e75b98f081f5cabde))
- **goals**: incremental-index's named checks run the tests they name ([`fa8b07a3`](https://github.com/novis-lang/novis/commit/fa8b07a3b628d191997883063d477b132dc9e4d0))
- **host**: a cancelled task unwinds without a crash in Windows' optimized builds ([`6b0bc591`](https://github.com/novis-lang/novis/commit/6b0bc591ce0f793bf528233e8496b34fd34fffae))
- **host**: the slice clock wakes on time on macOS, so a group's deadline is read before the CPU stop ([`48879136`](https://github.com/novis-lang/novis/commit/488791366020cd1f43d05e9a4b11898cceb2674b))
- **lsp**: the editor types a program against the built-in components ([`35d3753f`](https://github.com/novis-lang/novis/commit/35d3753fe6f2e0c5235303120366468eac5096d2))
- **nv**: a release judge reads build.yml's legs at the run's commit ([`60bf5449`](https://github.com/novis-lang/novis/commit/60bf5449fbce929d6f196fe8ea6b5484ade31214))
- **nv**: embedded text runs no platform matrix ([`0d9eda6a`](https://github.com/novis-lang/novis/commit/0d9eda6a7ee33fa769c90fb421b8fabe82854f4d))
- **nv**: the code lane covers every input the binary is built from ([`b5a12569`](https://github.com/novis-lang/novis/commit/b5a12569b6185c7afe4e5a8d3a89bafcdaeb8f55))
- **release**: a release binary names the commit it was built from, without -dirty ([`4ce8a689`](https://github.com/novis-lang/novis/commit/4ce8a6893fc6951470870f022d7f63d757600a98))
- **release**: publish fetches the committed lock before --apply bumps the version ([`53c7b504`](https://github.com/novis-lang/novis/commit/53c7b504e968ddaaac27b108149747d5db811e21))
- **runtime**: a Serialize payload refused partway frees every object decode built ([`4b8503ca`](https://github.com/novis-lang/novis/commit/4b8503ca80d7fb8ee67c02f4768bf0dfebcb3ee8))
- **site**: the front page names the site Novis-Lang to search engines ([`1684a158`](https://github.com/novis-lang/novis/commit/1684a1580b6c9026d58ed5237ff4ee83fce9e5fa))
- **site**: the front page's search questions are stamped against its new head ([`383af5ab`](https://github.com/novis-lang/novis/commit/383af5abc45d4659f0e35cdd722c338960196ae8))
- **site**: the hero slogan says SQL injection does not compile ([`d5294d15`](https://github.com/novis-lang/novis/commit/d5294d15a7339ff03c827c29e37b5ecf680cf074))
- **site**: the hero slogan says SQL injection is blocked, so it never reads as NoSQL injection ([`72b8c974`](https://github.com/novis-lang/novis/commit/72b8c9749abe6095f90bb4765f11c93c23c87f59))
- **spreadsheet**: the tests read back through calamine 0.38, clear of RUSTSEC-2026-0436 ([`56f215bb`](https://github.com/novis-lang/novis/commit/56f215bb0a1fe129b0e47fdf4bf1d887726f63bc))
- **types**: a per-file table part inherits the provenance switch and never turns it on ([`a35771e7`](https://github.com/novis-lang/novis/commit/a35771e7ca124c25619eb2b5c88d7c29dded36d3))
- **website**: the sidebar opens with the current page's entry in view ([`2dfacf60`](https://github.com/novis-lang/novis/commit/2dfacf6099c8f4c6068b1be838b5ced38f1f31df))

### Performance

- **lsp**: an answer waits for the open document, never for the index ([`e5b77942`](https://github.com/novis-lang/novis/commit/e5b7794256c67ea741d1f4ffac795ed65fc8b757))

### Internal

55 further commits: docs (24), build (10), test (10), chore (8), wip (2), refactor (1).

[Full changes](https://github.com/novis-lang/novis/compare/v0.0.1...v0.0.2) — 84 commits.

## [0.0.1] - 2026-11-02

This is the first release of Novis.

Novis is the programming language for web applications and the command line. You can build a
business app, a website, an API or any other web service with it. The tools that a modern web
application needs are built in.

- **Input is tainted.** Text from a request must be checked or escaped before it reaches a
  database, a shell or a page.
- **Every request on its own.** Each request has its own memory. When one request crashes, the
  others keep running.
- **One program.** `nvs` runs your code, serves web requests, runs your tests and formats your files.
- **Compiled to machine code.** Each file is compiled the first time it is needed. You never run a
  build step.

### Pre-Alpha

Novis is Pre-Alpha. Any release can change the language, and Novis is not ready for production use.

### Install

Download the archive for your system from the files below. Each archive contains the `nvs` program.
Put `nvs` on your `PATH`, then run `nvs --version` to check it. `SHA256SUMS` lists the checksum of
every file.

You can also use the Docker image:

```sh
docker pull ghcr.io/novis-lang/novis:0.0.1
```

The editor extension for VS Code is `nvs-0.0.1.vsix`, in the files below. In VS Code, open the
Extensions view and choose **Install from VSIX...**.

### Read more

- [Install Novis](https://novis-lang.org/install/)
- [The roadmap](https://novis-lang.org/in-depth/roadmap/): the work that is planned next.
- [What changed](https://novis-lang.org/in-depth/what-changed/): what is new in Novis.
