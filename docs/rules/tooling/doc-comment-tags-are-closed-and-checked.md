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
- `@see <path>[#<anchor>]` names a file in the project, and may name a heading inside a Markdown file.
  A target that contains `/` or `.` is a file target, and every other target is a member target. A
  class or member name never contains either, so the two forms never overlap, and
  `nvs_syntax::doc_target` is the one classifier the checker, the language server, `nvs pkg pack`,
  `nvs meta` and `nvs doc` all call.
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

**A `@see` file target.** The path is relative to the writing file's folder, as an `@example` path and
a `require` are (`rule:statements/require-is-the-only-inclusion-construct`). The target must be a regular
file, and it must stay inside the project.
- **The project** is the nearest folder at or above the writing file that holds `nvs.toml` or
  `package.toml`. A file with neither above it has its own folder as the project.
- **The path is judged before it is joined.** An absolute path, a drive or UNC prefix, and a `..` that
  climbs past the project are refused on the written text alone. After the join, the canonical path must
  still start with the canonical project folder, so a symlink cannot leave it either.
- **Any file may be a target, and only Markdown takes an anchor.** A `.md` or `.markdown` file takes
  `#<anchor>`, and an anchor on any other file is an error. The anchor must equal the slug of one heading
  in the file. Slugs follow GitHub's rules, which VS Code's Markdown preview shares: lowercase, every
  character that is not a letter, a digit, a space, `-` or `_` removed, each space turned into `-`, and a
  repeated slug suffixed `-1`, `-2` in order. A heading is ATX (`#` to `######`) or setext (a `===` or
  `---` underline), and nothing inside a fenced code block counts. The scan is `doc_target`'s own and
  reads no Markdown crate: the stdlib's parser is a run-time crate, and a heading scan is twenty lines.
- **A package carries its targets.** `nvs pkg pack` and the registry build refuse a package in which a
  `@see` file target or an `@example` path names a file outside the package's `[package] files`, and the
  refusal names the tag, the file and the line. The consumer's check then finds every target, so the rule
  "the target exists" holds the same way in a fetched package as in the project that wrote it.
- **`nvs meta --json` and `nvs doc`** write a file target as a link. `see` in `nvs meta --json` stays a
  list of strings, a file target written relative to the program root with `/` separators and its anchor
  (`rule:tooling/meta-json-takes-a-program`); `nvs doc` writes it as a Markdown link relative to the page.

**Every `@see` failure is one diagnostic, `E0323`,** with a message and help for each case: no such class
or member, no such file, no such heading, an anchor on a file that is not Markdown, and a path that
leaves the project. A miss carries a suggestion that replaces the written name with the closest one,
measured by edit distance, when one is within a third of the written length: the closest member of the
class, or the closest heading slug of the file. A missing file gets no suggestion. The editor's quick fix
is that suggestion, translated (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
The checker records what each `@see`, `@example` and `@throws` target resolved to, and every editor
feature reads that record (`rule:ide/the-request-set-is-closed`).

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
`nvs doc` and the language server, and the AST holds two more spans per tag line. A `@see` file target
costs the check one `stat`, as `@example` does. An anchored target also reads the Markdown file once and
scans its headings; the text is dropped when the tag is checked, and the read is recorded as a footprint
file so test selection reruns a case when the Markdown file changes. The language server keeps one
resolved target per tag for as long as the file's analysis is kept, and it rechecks an open `.nvs` file
when that file is edited, not when the Markdown file changes; `nvs check` is the authority.

**Declined:**
- **Prose only, with no tags.** A Markdown link and a fenced block buy no check, and signature help has no
  sentence per parameter.
- **A type in the tag**, as in PHPDoc. The signature is the one place a type is written, and a second copy
  drifts.
- **A warning for every undocumented parameter.** A required tag is answered with generated noise. The
  warning fires only where the author already documented the others.
- **Swift's Markdown list form** (`- Parameter x:`). A reader cannot tell a structured list from an
  ordinary one.
- **A URL as a `@see` target.** The checker never touches the network, so a URL cannot be checked, and
  every tag is checked. A Markdown link in the prose already carries an unchecked URL.
- **A line anchor, `#L42`.** It points at the wrong line after the next edit above it.
- **An HTML anchor, `<a id="x">`, as a target.** Only headings: one kind of anchor, one scan.
- **A sixth tag such as `@link`.** `@see` already means "related", and PHPDoc and Javadoc readers expect
  it to take a document.
- **Skipping the tag checks for a fetched package's files.** The consumer's hover and `nvs doc` would
  then link to a file that is absent.
- **Dart's checked references in prose only.** They buy a check, but signature help still has no sentence
  per parameter.
