An extension is loaded from an `[[extension]]` entry in the root-owned `nvs.toml`, and from nowhere
else:

```toml
[[extension]]
path   = "image.nvsx"
sha256 = "…"
```

A project cannot cause code to be loaded. The pin is a field of the entry rather than a naming
convention over a repeated key, which is one reason the configuration format has an array-of-tables
shape (`rule:config/lists-are-arrays-and-repeated-records-are-arrays-of-tables`); the entry lives
where `rule:config/ownership-is-the-trust-boundary` puts every grant of authority.

The set is reloadable, not boot-only. A reload re-verifies every pin against the file on disk, loads
the manifests, and refuses the whole swap if any pin does not match — so a running server gains, loses
or replaces an extension without dropping a request, and never on a binary that changed under its pin.
Duplicate class names across extensions are refused at load, which is what makes the set's hash
order-independent, and the set is folded into every compiled unit's key
(`rule:config/the-extension-set-is-in-every-unit-key`) so a changed set is a lazy recompile with no
invalidation pass. A component is compiled once, into the same content-addressed artifact cache as
Novis's own code, and the compiled module is shared across every core.
