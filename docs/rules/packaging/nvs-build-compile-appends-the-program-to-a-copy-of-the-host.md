`nvs build --compile entry.nvs -o app[.exe]` produces a portable single-file executable. It resolves the
entry file's `require` graph (`rule:packaging/a-bundled-require-resolves-at-build-time`), appends the
resulting source list and a footer to a **copy of the current `nvs` host binary**, and writes the result.
The output runs on any machine matching the host binary's own target, and it is one file: no sidecar
archive, no interpreter to install.

**The shipped executable never mutates itself.** "End users can rebundle it easily" means the app author
reruns this one command over their own source, the way `bun build --compile` or `cargo build` is rerun —
never that a running end-user copy accepts new code and re-emits itself. That is what makes a bundle a
single trust domain (`rule:programs/bundle-trust-domain`): the only principal who ever produces the
artifact is the one who already has the source, and a bundle may not install itself as a service for
the same reason.

The feature ships a runnable *program*; `nvs serve` is not bundled, and the bundler is not a package
manager (`rule:packaging/build-compile-packages-what-is-on-disk-and-resolves-nothing`). The only
genuinely new code is the footer writer and the footer reader
(`rule:packaging/a-bundle-is-found-by-its-footer-before-argv-is-read`); everything after that is the
ordinary `nvs run` pipeline.

**Declined:**
- **A sidecar archive beside the executable.** It needs no footer code and no macOS signing step, but
  it is two files, which is the distribution every scripting language already has.
- **A shipped executable that rebundles itself.** It would make a build artifact mutable at rest,
  re-sign it on every rebundle instead of once at build time, and raise the question of who may
  trigger a rebundle and as which identity. The author already has the source and reruns the build.
