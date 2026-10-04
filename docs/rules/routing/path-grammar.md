A path is a string literal beginning at the root, validated while checking, whose segments are
fixed segments or captures:

- **`{name}`** captures one whole segment.
- **`{name?}`** captures one whole segment **or none**. Last position only, at most once, never in
  the same path as a `{name...}`, and the parameter it binds must have a default — that is what makes
  the absent case well-typed rather than nullable by accident; a `{name?}` bound to a parameter
  without one is a compile error naming both. `/posts/` does not match `/posts/{page?}`: an empty
  final segment is not an absent one (`rule:errors/ambiguous-input-refused`).
- **`{name...}`** captures every remaining segment as one `tainted string`, last position only.
- Everything else is a fixed segment, compared byte for byte and case-sensitively
  (`rule:classes/names-resolve-case-sensitively`).

A capture is a whole segment: `/u{id}` and `/{id}.json` are refused, not partially matched. Every
capture names a parameter of the method it is attached to, and that parameter's type is what the
segment converts to (`rule:security/route-capture-is-laundered-by-its-type`). The inline `{id:uint}`
form is deliberately not taken: the type is already written on the parameter, and a constraint stated
in two places is one that can disagree.

`{name}` rather than `:name`, for two reasons: `:` is a legal character inside a path segment
(`/a:b` is a valid path), so `:name` would need an escape rule the braced form does not; and the
braced form is what OpenAPI, Laravel, Symfony, axum and ASP.NET all use, so it is the spelling a
reader arrives with.
