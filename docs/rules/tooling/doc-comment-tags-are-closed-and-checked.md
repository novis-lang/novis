The body of a doc comment is Markdown. Five tags may appear, each at the start of its own line:

```nvs
/// Charges a card, in the smallest unit the currency has.
///
/// @param $cents the amount, in cents
/// @param $card the card to charge; it must not be expired
/// @return the ledger id of the charge
/// @throws CardDeclined when the bank declines the card
/// @see Core\Money::fromCents
/// @example examples/charge.nvs
```

**The set is closed, and every tag in it is checked.** Any other `@tag` at the start of a line is a
diagnostic, `E0128`, and that one sentence is the entire difference between a closed set and a convention:
an unenforced set grows, and the growth is how PHPDoc arrived at documenting a signature twice. An `@`
anywhere else in the prose is just a character. `@returns` gets its own help and a suggestion that rewrites
it to `@return`. `@var` has nothing to say, since every binding is annotated; `@deprecated` is an attribute
(`rule:attributes/a-deprecation-names-its-replacement-as-code`); `@since` and `@internal` are history and
visibility, which the package and the modifiers already state (`rule:security/capability-check-at-the-door`
covers "this touches the filesystem"); `@inheritdoc` is a line that says nothing, because a member without
a comment of its own inherits one (`rule:tooling/strict-docs`). Each stays an error.

**The shapes.**
- `@see <member>` names a class, member, enum or constant, and must resolve.
- `@example <path>` names a file that must exist **and must be inside a directory the test corpus walks**,
  so an example that stops compiling fails the build rather than rotting in a page.
- `@param $name text`: `$name` comes directly after the tag. A variadic parameter is written `$name` too,
  without `...`.
- `@return text`.
- `@throws Name text`: `Name` resolves like a type name in that file, `use` lines included.

The text of `@param`, `@return` and `@throws` is Markdown and runs to the end of the line. A following
`///` line that is not a tag and is not blank continues the text of the tag above it, so a long sentence
can wrap. Each continuation line stays a line of the text, so a `- ` line is a list item: that is how a
shape parameter's keys are listed under its `@param`, and hover and `nvs doc` show the list inside the
parameter's own item. The text is required: a tag with no text is an error that says to write what the parameter means.
That is the error a newly inserted `@param` line shows until its author writes the sentence.

**A tag never carries a type.** The signature is the one place a type is written, and a second copy drifts.
- After `@param`, anything but `$name` is an error, the parser's: `@param int $x` and JSDoc's
  `@param {int} $x`. Its suggestion deletes the type.
- If the first word of the text is the declared type, it is an error, the checker's, with a suggestion that
  deletes the word: the parameter's type after `@param $x`, and the return type after `@return`. A union or
  nullable type counts if the word is the whole type or one of its members. `null` alone is never flagged,
  because "null when the card is declined" is a normal sentence, and a word that only looks like some other
  type is never flagged.
- `@throws Name` is not a repeated type: a signature has no `throws` clause.

**Where each tag may go.** `@param` and `@throws` go on a function, a method (abstract and interface ones
included) or a constructor. `@return` goes on a function or method whose declared return type is neither
`void` nor `never`, and never on a constructor. Anywhere else each of the three is an error naming the
declaration kind. An anonymous function is not a declaration and carries no doc comment.

**What the checker holds each tag to.** Every one of these errors carries a suggestion, so its quick fix is
a translation (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
- `@param $name` names a parameter of this declaration, at most once. If the name is unknown and exactly
  one parameter has no `@param`, the suggestion renames the tag to it: that is the fix after a parameter
  was renamed by hand. Otherwise the suggestion deletes the line. A second `@param` for one parameter, and a
  second `@return`, are deleted by theirs.
- `@throws Name` resolves to a class that is a `Throwable`, and names each class at most once. The body is
  not checked against it: Novis has no checked exceptions, and the tag documents intent.
- **A missing `@param` is a warning only where the same comment already has at least one `@param`.** Its
  suggestion inserts `/// @param $name ` after the `@param` of the parameter before it in signature order,
  or before the first `@param`. A comment with no `@param` at all is never warned, for
  `rule:tooling/strict-docs`'s reason, and a fix never writes a sentence.

**Every tag repeats where it can, and the order is free.** A declaration carries as many `@see`, `@example`
and `@throws` lines as it needs. `@see` and `@example` are read back in the order they are written, as lists
in `nvs meta --json` (`rule:tooling/meta-json-takes-a-program`). `@param` is read by parameter: `nvs doc`,
`nvs meta --json`, hover and signature help show signature order whatever order the lines are in, and
`nvs fmt` writes the lines in that order (`rule:tooling/fmt-orders-the-doc-tags`). A user member's card has the `Core` card's shape
(`rule:core-api/reference-card`): `short`, `params` as `{name, desc}` in signature order, `return`,
`errors` as `{name, desc}`, `see` and `example`.

**Inheritance.** A member with no `///` of its own inherits the whole comment
(`rule:tooling/strict-docs`). An inherited `@param` is shown under the overriding member's parameter at the
same position, because an override may name its parameters differently. A member with its own comment
shows only its own tags. A promoted constructor parameter's `@param` is its property's documentation.

**Memory:** nothing at run time, in speed or in memory. The tags are read by the checker, `nvs meta`,
`nvs doc` and the language server, and the AST holds two more spans per tag line.

**Declined:**
- **Prose only, with no tags.** A Markdown link and a fenced block buy no check, and signature help has no
  sentence per parameter.
- **A type in the tag**, as in PHPDoc. The signature is the one place a type is written, and a second copy
  drifts.
- **A warning for every undocumented parameter.** A required tag is answered with generated noise. The
  warning fires only where the author already documented the others.
- **Swift's Markdown list form** (`- Parameter x:`). A reader cannot tell a structured list from an
  ordinary one.
- **Dart's checked references in prose only.** They buy a check, but signature help still has no sentence
  per parameter.
