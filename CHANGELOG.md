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
