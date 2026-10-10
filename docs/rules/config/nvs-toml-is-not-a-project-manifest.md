`nvs.toml` is a root-owned deployment file. It is **never searched for by walking up from a source
file**, it holds no source-tree state, and an `[autoload]` table is refused as an unknown key for the
reason `rule:programs/no-runtime-autoload` gives: the rejection of "a manifest found by walking up from
the entry file" stands, and it is about discovery and lifetime rather than syntax. The autoload map is
a property of the source tree, while `nvs.toml` is deployment state: it is absent from
`nvs build --compile` and from an editor's `nvs check`, and a map in it would make one source tree
behave differently under two hosts, which the artifact cache would then have to key on. Sharing an extension
with `Cargo.toml` is not a collision; the name, the location and the owner all differ. A package may
not hold an `nvs.toml` (`rule:packaging/a-package-cannot-reach-its-host`), and a working directory is
not a package: committing a development `nvs.toml` to a project is fine, and deploying it is what
`--config` is for.

Where the file *is* found is `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`:
a `--config` list, else `./nvs.toml` in exactly one directory, else the data folder's `nvs.toml`. That
single-directory step is deliberately one step short of a walk, and the step is not taken. The data
folder's file is Novis's own default, one per data folder and not per project. What makes a file in a
working directory safe to read there is `rule:config/ownership-is-the-trust-boundary`, the resolved
absolute path announced at boot, and the installer storing an absolute `--config` for every service.
