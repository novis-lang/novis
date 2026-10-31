Two things a package may not do to the tree it lands in.

**A package may not contain an `nvs.toml`.** Server configuration is root-owned and
deployment-scoped (`rule:config/nvs-toml-is-not-a-project-manifest`); a package that shipped one
would be asking to configure the host it lands on. Fetching an archive containing one is an error
naming the file, and the registry refuses such an archive at publish.

**A package's files are reachable only from within the package.** A `require` or an `autoload` path
inside a package that escapes the package's own directory — through `..`, an absolute path or a
symlink — is a **compile error**. A package cannot read the application's tree, and the check is the
same canonicalise-and-prefix-check `spawn script` performs against its granted roots
(`rule:security/path-scope-canonicalise-then-prefix`).

The path half is `E0661`: each `require` edge and each existing `autoload` root written in a package
file is canonicalized and must lie under the package's directory (`crates/nvs-types/src/package.rs`).

`nvs pkg pack` refuses a file named `nvs.toml`, in any case, anywhere in the archive, and
`nvs pkg fetch` refuses an archive that holds one before the package's folder is replaced. Both call
one check, `pack::check_not_server_config` (`crates/nvs-cli/src/package/pack.rs`), so an archive from
a mirror or a private registry is held to what the registry's build refuses. The refused fetch
removes the files it unpacked and writes no lock (`crates/nvs-cli/tests/package_fetch.rs`).
