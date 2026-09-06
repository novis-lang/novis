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
