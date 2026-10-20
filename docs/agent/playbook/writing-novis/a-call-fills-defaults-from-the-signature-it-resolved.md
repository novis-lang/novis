- **A call fills a parameter's default from the signature it resolved, never from the method that
  runs.** So a call through a parent's type to an override that adds a defaulted parameter passes
  nothing for it, and the override reads a null string and panics. Before deciding that an override
  form "is allowed", run it through the parent's type, not only through the subclass's. That is why
  `rule:classes/an-override-keeps-its-parents-signature` keeps the count exact. See
  `crates/nvs-types/src/conformance.rs`. `until: gone crates/nvs-types/src/conformance.rs:own.params.len() != parent.params.len()`
