`#[Property]` runs a method against generated inputs, and the generators are **derived from the
parameters' declared types** by the same declaration walk a codec is derived from, user classes
included. That is why this belongs to the compiler rather than to a package: only a compiler can
walk a declared type, and a userland version needs a hand-written generator per type, which is the
boilerplate that keeps property testing rare.

A failure shrinks to a minimal counterexample and reports the seed that reproduces it exactly. A
`gen:` option narrows one parameter where its declared type is wider than the domain. A generated
`string` is valid UTF-8 including combining marks and grapheme clusters, because that is what a
`string` is — a property that only holds for ASCII is a property that is not true.
