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

**On disk.** A manifest's `"sink": true` parameter and `"source": true` method are both checked
(`tests/conformance/reject/a-tainted-argument-to-an-extension-sink-does-not-compile.nvst`,
`tests/conformance/ext/an-extension-declared-source-taints-its-result.nvst`).

**Declined:**
- **Sinks only, with no source declaration.** An extension that fetches remote data or decodes an
  untrusted file would return plain output from plain arguments, so untrusted bytes would enter the
  program clean.
