A manifest may say two things, and only these two. **A parameter refuses `tainted`** — the extension
is a sink for that argument, and a tainted operand is rejected there exactly as at a query text
parameter. **A return is always `tainted`** — the extension is a source, producing bytes Novis did not
see enter, so the result is tainted even when every argument was plain.

Both are restrictions (`rule:security/extension-manifest-only-tightens`): the first can only make a
call site fail that would otherwise have compiled, and the second can only add a qualifier the caller
must then launder. The axis exists only in the manifest, for the compiler — a guest's generated
bindings ignore both, since neither affects the wire representation.

**On disk.** A manifest's `"sink": true` parameter and `"source": true` method are both checked
(`tests/conformance/reject/a-tainted-argument-to-an-extension-sink-does-not-compile.nvst`,
`tests/conformance/ext/an-extension-declared-source-taints-its-result.nvst`).
