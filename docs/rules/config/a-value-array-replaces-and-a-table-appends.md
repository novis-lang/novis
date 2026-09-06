`key = [...]` is one value, and a later value wins — so `[debug] mode`, `[capabilities] script.spawn`
and `debug.trace` are **replaced wholesale** by a later file. `[[extension]]`, `[[schedule]]`,
`[[server.mount]]`, `[[app]]` and `[[include]]` entries **accumulate** across the tree.

The split is not a special case: it is what each shape already means inside one file. Two
`[[schedule]]` blocks in one file are two schedules, so appending across files extends a rule rather
than adding one, and replacing would make the cross-file behaviour of the syntax differ from its
within-file behaviour. For a capability the direction also matters on its own: replacement means **the
last file that mentions a grant states the whole grant**, and no reader has to assemble the effective
root list from four files to know what it is.

A third array shape that fits neither half is the thing that would reopen this.
