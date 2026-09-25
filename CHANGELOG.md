# Changelog

Every released version of Novis, newest first. Generated from the commit log by
`bun nv release` and prepended by the release workflow -- edit a section only to correct it,
never to add one by hand.

What a version number promises is [ADR 0068](docs/decisions/0068.md)
§§ 2-3: it covers the language, the `Core` library, `nvs.toml`, the CLI, diagnostic identity, the
extension ABI and `serialize()` output -- and explicitly not the Rust APIs of the `nvs-*` crates.
Before 1.0 the breaking slot moves left: `0.MINOR` carries breaking changes.

A release is cut by [the release workflow](.github/workflows/release.yml), fired by hand from the
Actions tab; [docs/release.md](docs/release.md) is the procedure and what it needs configured.
