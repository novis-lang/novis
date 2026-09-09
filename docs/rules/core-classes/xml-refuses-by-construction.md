`Core\Xml` refuses a `<!DOCTYPE …>` whole and resolves no entity beyond the five predefined ones, so
the three classic XML attacks are **absent code paths rather than bounded ones** — there is no flag to
have defaulted to off, because there is nothing for a flag to have switched.

Each of the three is answered by subtraction rather than by a limit:

- **External entity resolution** does not exist. One function in the module resolves a reference, its
  vocabulary is closed at `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;` and numeric character references,
  and no branch anywhere reaches a filesystem, a network or another document. This is
  `rule:security/no-runtime-grant` applied to a parser: the authority is not withheld, it was never
  wired.
- **A DTD naming an external subset** is refused at the `<!DOCTYPE` token, before the identifier is
  read, so the refusal cannot depend on telling a `SYSTEM` identifier from a `PUBLIC` one.
- **A billion-laughs expansion** needs an internal subset to declare its entities in. There is no
  internal subset, so there is no expansion, so there is no expansion factor to meter — this is closed
  at the declaration rather than metered by `rule:core-classes/decompression-bound`'s ratio and ceiling,
  and refusal is one comparison where metering would be a DTD parser, an entity table, a substitution
  pass and a bound over it.

The five predefined entities and numeric character references still expand, and they are not an
exception: they are the document's own characters written another way, they resolve without consulting
anything, and refusing them would be refusing well-formed XML rather than refusing an attack.

**Depth is the one bound that stays a number.** Element nesting is written by a document rather than
declared by it, so there is no token to refuse: `DEPTH_CEILING` caps how deeply elements may nest, at
the same figure `Core\Json::decode` carries and for the same reason — a document engineered to be deep
costs work before any value exists, and a stated bound is one a caller can reason about. The parse loop
is iterative, so it is a policy rather than a guard over the native stack.

A program that needs to fetch something fetches it with `Core\Http\Client` and parses the bytes it got
back. Both refusals are `ParseError` throws naming what was written, per
`rule:errors/ambiguous-input-refused`; `Core\Html`'s WHATWG entry is the door for input that is
HTML-shaped rather than well-formed, and it never fails (`rule:core-classes/html-parsing`).
