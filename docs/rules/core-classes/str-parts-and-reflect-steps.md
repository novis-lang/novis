`Core\Str::parts(string<K> $text): array<string|K>` gives the text pieces and the holes of a
`string<K>` (`rule:types/string-holes`) in order. A text piece has `{{` and `}}` already turned into one
brace, so a library that joins the pieces never handles an escape. A text piece of a `tainted string<K>` is a `tainted string`; a hole is a `K`, with no qualifier.

`Core\Reflect::steps(propertyPath<T> $path): array<Core\Reflect\PropertyInfo>` gives one row per
segment of a path (`rule:types/property-path`), in order: the description `Core\Reflect` already
returns for a property (`rule:core-classes/reflect`), with its name, visibility and declared type, so a
library sees whether a step is one object, a nullable one or an array. No new class is added.

`Core` parses only the holes. The text between them is the library's own language — SQL, a filter
syntax, a log template — and a query builder is not `Core` (`rule:core-classes/db-one-api`). Both
members run once per call, at run time, and neither caches: a `string<K>` literal is split again on each
call.

The pair exists because the types are only half of the feature. A library that takes a `string<K>`
condition has to turn each hole into a column and each text piece into query text, and a library that
takes a `propertyPath<T>` sort field has to know which steps are relations. Both answers are facts the
compiler checked, so `Core` hands them over typed, and the library never parses braces or walks a
class by name.

- **Memory:** `parts` allocates its result array and one string per piece, per call, charged to the
  request. `steps` allocates its result array and one description per segment, per call. Nothing is
  kept after the call returns.

**Declined:**
- **Splitting a `string<K>` literal while compiling, for now.** It is decided only if the bench for
  `parts` shows the split costs more than the work a library does around it.
- **A new class for a step, `PathStep`.** `PropertyInfo` already has every field a step needs, and a
  second class would be a second description of the same property.
- **`Core` turning the pieces into SQL.** It would make `Core` a query builder and parse the text between
  the holes, which is the library's language (`rule:core-classes/db-compile-time-query-checking`).
