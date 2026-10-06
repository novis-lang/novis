The body of a doc comment is Markdown. Two tags may appear, each on its own line in a trailing block:

```nvs
/// The price in cents, never a float.
///
/// @see Core\Money::fromCents
/// @example examples/charge.nvs
```

`@see <member>` names a class, member, enum or constant, and must resolve or it is a diagnostic.
`@example <path>` names a file that must exist **and must be inside a directory the test corpus walks**,
so an example that stops compiling fails the build rather than rotting in a page. **Any other `@tag` at
the start of a line is a diagnostic**, and that one sentence is the entire difference between a closed set
and a convention: an unenforced set grows, and the growth is how PHPDoc arrived at documenting a signature
twice. An `@` anywhere else in the prose is just a character.

**Both tags repeat, and every one of them is checked.** A declaration carries as many `@see` and
`@example` lines as it has cross-references and examples, in whatever order they are written, and the
order they are written is the order they are read back: `nvs meta --json` emits `see` and `example` as
lists (`rule:tooling/meta-json-takes-a-program`) and `nvs doc` renders each list on one line. A second
tag is not a weaker one — a `@see` that does not resolve is the same diagnostic whether it is the first
line of the block or the last.

There is no `@param`, `@return`, `@throws`, `@var`, `@deprecated`, `@since` or `@internal`, because almost
nothing PHPDoc carried survives as prose. Parameter and return types are the signature, which cannot drift
from itself; a callable carries its own (`rule:types/callable-signature`); what a `Core` member throws is
its registry card (`rule:core-api/reference-card`) and for user code is a sentence; every binding is
annotated, so `@var` has nothing to say; deprecation is an attribute (`rule:attributes/inert-metadata`);
"this touches the filesystem" is a declared capability (`rule:security/capability-check-at-the-door`). A
parameter that needs explaining is named in a sentence — "the timeout is in **milliseconds**" — and the
diagnostic says so: *`@param` is not a documentation tag; name the parameter in a sentence instead — its
type is in the signature*, and likewise for `@throws` and `@returns`.

The two tags survive because each buys a *check* — a cross-reference that must resolve, an example that
must still compile — and a third has to meet the same standard, never merely improve a rendering.
