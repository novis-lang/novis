`nvs.toml` is a root-owned deployment file. It is **never searched for by walking up from a source
file**, it holds no source-tree state, and an `[autoload]` table is refused as an unknown key for the
reason `rule:programs/no-runtime-autoload` gives: the rejection of "a manifest found by walking up from
the entry file" stands, and it is about discovery and lifetime rather than syntax. Sharing an extension
with `Cargo.toml` is not a collision; the name, the location and the owner all differ.

Where the file *is* found is `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`:
a `--config` list, else `./nvs.toml` in exactly one directory. That single-directory step is
deliberately one step short of a walk, and the step is not taken. What makes a file in a working
directory safe to read there is `rule:config/ownership-is-the-trust-boundary`, the resolved absolute
path announced at boot, and the installer refusing a service whose configuration came from a working
directory.
