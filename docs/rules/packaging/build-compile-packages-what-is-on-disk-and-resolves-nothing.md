`nvs build --compile` packages what is already resolved and present on disk, and nothing else. It
fetches no dependency, reads no lockfile and contacts no registry; a `require` target that is not on
disk fails the build (`rule:packaging/a-bundled-require-resolves-at-build-time`) rather than being
looked for anywhere.

Resolving dependencies into source on disk is a separate command's job — `nvs pkg` — and the two
compose rather than overlap: `nvs pkg install && nvs build --compile`. Keeping them apart is what keeps
the bundler's own surface at one input, one output and one footer.
