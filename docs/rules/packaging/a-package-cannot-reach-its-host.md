Three things a package may not do to the tree it lands in.

**A package may not contain an `nvs.toml`.** Server configuration is root-owned and
deployment-scoped (`rule:config/nvs-toml-is-not-a-project-manifest`); a package that shipped one
would be asking to configure the host it lands on. Fetching an archive containing one is an error
naming the file, and the registry refuses such an archive at publish.

**A package carries one `package.toml`, its own, at its root.** A second one anywhere below the root,
in any case of letters, is refused like an `nvs.toml`. The compiler finds a file's package by the
`package.toml` above it (`rule:packaging/a-package-file-stays-under-its-declared-prefix`), so a
second manifest inside a package would hand part of the package's tree to another name, or to none.
A package that keeps a second manifest as data renames the file.

**A package's files are reachable only from within the package.** A `require` or an `autoload` path
inside a package that escapes the package's own directory — through `..`, an absolute path or a
symlink — is a **compile error**. A package cannot read the application's tree, and the check is the
same canonicalise-and-prefix-check `spawn script` performs against its granted roots
(`rule:security/path-scope-canonicalise-then-prefix`).

The path half is `E0661`: each `require` edge and each existing `autoload` root written in a package
file is canonicalized and must lie under the package's directory (`crates/nvs-types/src/package.rs`).

`nvs pkg pack` refuses a file named `nvs.toml`, in any case, anywhere in the archive, and every
`package.toml` but the root one. `nvs pkg fetch` refuses an archive that holds one before the
package's folder is replaced, and a `[[download]]` entry may not be placed at such a path. All of them
and the registry's build call one check, `pack::check_reserved_file`
(`crates/nvs-cli/src/package/pack.rs`), so an archive from a mirror or a private registry is held to
what the registry's build refuses. The refused fetch
removes the files it unpacked and writes no lock (`crates/nvs-cli/tests/package_fetch.rs`).
