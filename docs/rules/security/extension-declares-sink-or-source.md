A manifest may say two things, and only these two. **A parameter refuses `tainted`** — the extension
is a sink for that argument, and a tainted operand is rejected there exactly as at a query text
parameter. **A return is always `tainted`** — the extension is a source, producing bytes Novis did not
see enter, so the result is tainted even when every argument was plain.

Both are restrictions (`rule:security/extension-manifest-only-tightens`): the first can only make a
call site fail that would otherwise have compiled, and the second can only add a qualifier the caller
must then launder. The axis exists only in the manifest, for the compiler — a guest's generated
bindings ignore both, since neither affects the wire representation.

Extensions are most useful exactly where these two apply: a template engine, a query builder, an LDAP
filter builder and a Markdown renderer are sinks, and an extension that fetches or decodes remote data
is a source.

**A manifest that requests any I/O makes every method a source.** A manifest whose `requests` names a
`read`, `write` or `connect` entry can return bytes from a file or a host the program never read, so
each of its returns is `tainted` whatever its methods declare (`nvs_ext::manifest::Manifest::is_source`).
The request decides, not the operator's grant: a program's types do not change with its configuration,
and an entry with no `grants` returns the same types as one with them.

**On disk.** A manifest's `"sink": true` parameter and `"source": true` method are both checked
(`tests/conformance/reject/a-tainted-argument-to-an-extension-sink-does-not-compile.nvst`,
`tests/conformance/ext/an-extension-declared-source-taints-its-result.nvst`), and so is a manifest
that requests I/O (`crates/nvs-types/tests/extensions.rs`).

**Declined:**
- **Trusting `"source"` alone.** A manifest that reads files but forgot `"source": true` returned
  their bytes untainted, so an omission in a third party's manifest let outside text reach a sink.
- **Deciding by the grant.** The operator's grant can change at a reload, and a program's types would
  change with it.
- **Sinks only, with no source declaration.** An extension that fetches remote data or decodes an
  untrusted file would return plain output from plain arguments, so untrusted bytes would enter the
  program clean.
