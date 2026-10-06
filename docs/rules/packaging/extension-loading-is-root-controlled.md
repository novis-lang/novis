An extension is loaded from an `[[extension]]` entry in the root-owned `nvs.toml`, and from nowhere
else, and every entry carries its pin:

```toml
[[extension]]
path   = "geo.nvsx"
sha256 = "…"                      # required: 64 hexadecimal digits
```

A project cannot cause code to be loaded. The pin is a field of the entry rather than a naming
convention over a repeated key, which is one reason the configuration format has an array-of-tables
shape (`rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables`); the entry lives
where `rule:config/ownership-is-the-trust-boundary` puts every grant of authority. An entry without a
pin, or with one that is not 64 hexadecimal digits, is refused at boot and at reload, naming its file
and line, and `nvs ext pin <file>` prints the entry ready to paste. Signatures are not offered: a pin
of the exact bytes, written by root, is the stronger check for a file already on the disk.

Load refuses, each naming the entry: a file whose digest differs from its pin, a component that does
not validate, a manifest that is malformed or does not match the exports, an import outside the world,
a class under `Core\` or `Novis\` (`rule:packaging/the-first-party-components-are-built-in`), and a
class name another loaded extension already declares.

The set is reloadable, not boot-only. A reload re-verifies every pin against the file on disk, loads
the manifests, and refuses the whole swap if any entry fails — so a running server gains, loses or
replaces an extension without dropping a request, and never on a binary that changed under its pin.
Refusing duplicate class names is what makes the set's hash order-independent, and the set is folded
into every compiled unit's key (`rule:config/the-extension-set-is-in-every-unit-key`) so a changed set
is a lazy recompile with no invalidation pass. A component is compiled once, into the same
content-addressed artifact cache as Novis's own code, and the compiled module is shared across every
core.

**Not on disk.** The entry's shape is enforced: `nvs_config::extension::validate` refuses an entry
with no `path`, no `sha256` or a pin that is not 64 hexadecimal digits, at boot and at reload, naming
its file and line (`E0651`), and the pins fold into `env_hash`. Nothing reads the file yet, so no
digest is compared and none of the load refusals runs, and `nvs ext pin` does not exist.
