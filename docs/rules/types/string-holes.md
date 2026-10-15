`string<K>` is a string with holes: text in which every `{…}` is one value of `K`, checked where the
string is written. `K` is a `property<T>`, a `propertyPath<T>` (`rule:types/property-path`), or a set
of allowed string values (`rule:types/single-value-types`). `string<K>` is not a collection, as
`array<K>` is: it is one string, and `K` says what its holes may contain. `string` takes `<` as a type
word only when `<` follows it, the way `property` does, so a plain `string` is untouched.

A hole is `{`, one value of `K`, and `}`, with no spaces inside. `{{` is a literal `{` and `}}` a literal
`}`. An empty hole, an unclosed `{` and a lone `}` are errors, at the brace. Holes work in both quote
styles, because a brace with no `$` after it is already plain text in a string
(`tests/conformance/lang/a-string-literal-escapes-and-interpolates-exactly-what-each-quote-allows.nvst:61`).
A literal at a parameter typed `string<K>` is checked where it is written
(`rule:types/a-literal-at-a-key-parameter-is-checked-where-it-is-written`): a wrong hole is the
unknown-member diagnostic for its segment, placed on that segment. A double-quoted string that
interpolates `{$…}` is not a literal, so it needs `as string<K>`, which checks every hole at run time
and throws at the first bad one. `as ?string<K>` yields `null` there. `string<K> → string` is total and
gives the text as written, braces and escapes included.

**Taint stays on the text.** `as string<K>` does not strip `tainted`, unlike `as property<T>` and `as
propertyPath<T>`: the text between the holes is still the caller's text. A `tainted string` converts
to a `tainted string<K>`, and its text pieces are `tainted string` when `Core\Str::parts` splits them
(`rule:core-classes/str-parts-and-reflect-steps`). Only the holes are stripped, because each is a
declared name. A condition built from a request therefore never reaches `Core\Db`'s query text, which
refuses `tainted` (`rule:core-classes/db-one-api`). This is the security half of the type.

It is its own equality domain: two values are equal when their text is equal, and ordering one is
refused (`rule:types/ordering`).

The type exists because a library that takes a condition, a sort order or a column list as text — a
query builder, a filter, a report — names fields inside that text, and today the compiler sees only a
string. The holes are the part the compiler can check, and Novis parses nothing else: the text between
them is the library's own language.

- **Memory:** a value is one string, its text as written. Checking a literal costs nothing at run time.
  A run-time conversion walks the text once and allocates nothing past the result.

**Declined:**
- **A new word — `format`, `template`, `placeholder`, `path`, `text`.** `Core\Str::format` is the
  `printf` member, `template` reads as an HTML template, a SQL query's `:0` and `?` are placeholders
  (`rule:core-classes/db-parameters`), and `path` and `text` already mean something. `string<K>` adds no
  word and says what the value is.
- **Stripping `tainted` with the holes.** The text between the holes is the caller's, so a request
  could write SQL into a condition and have it reach query text as trusted.
- **Spaces inside a hole, `{ name }`.** It is a second spelling of the same hole and makes `{ ` in plain
  text ambiguous.
