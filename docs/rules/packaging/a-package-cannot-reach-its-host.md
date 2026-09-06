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

**Not on disk.** No fetch path exists to refuse an archive, and the compiler has no file-to-package
map against which to check a path.
