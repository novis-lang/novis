A `secret` value passed to an extension is a compile-time diagnostic, and no manifest may declare a
`secret` return. The refusal is in **both directions** and admits no declaration that softens it.

It is the same refusal that applies to serialization and to isolate crossing
(`rule:security/secret-crosses-no-boundary`), for the same reason: the value is copied into memory
whose subsequent handling Novis cannot reason about. Where an extension genuinely must see a
credential — a signing key for a protocol component — the conspicuous reveal is the way to say so at
the call site.

This constrains what may live outside `Core`. A signer built as an extension would need a reveal at
every call site, turning a deliberately conspicuous escape hatch into boilerplate, which is one of the
two reasons the protocol roster stays in `Core` (`rule:security/protocol-roster`).

**On disk.** A `secret` argument is refused at every extension parameter
(`tests/conformance/reject/a-secret-argument-to-an-extension-does-not-compile.nvst`), and a manifest
declaring a `secret` return does not load (`crates/nvs-ext/tests/load.rs`).
