Inside a string literal at a parameter typed `propertyPath<T>`, and inside a hole of a
`string<K>` literal, the completion list offers `T`'s public declared properties with their types, and
each `.` after a valid prefix offers the properties of the class that prefix reaches. Depth is
unlimited, and a path that loops back to a class it already passed costs nothing extra, because one
class's roster is listed per request.

This changes two shipped rules. `rule:ide/completion-offers-only-what-the-compiler-derived` offers a
parameter typed `property<T>` alone its names already, because a literal there compiles
(`rule:types/property-key`), and this extends that list to a path's segments.
`rule:ide/completion-is-asked-where-a-spelling-ends` gains `{` as a
trigger character (`crates/nvs-lsp/src/capabilities.rs`), answered only inside a `string<K>` literal,
where it opens a hole; a quote at a `propertyPath<T>` parameter opens the list, and
`.` inside such a literal starts the next segment.

Hover on a segment shows its property, with its class and declared type. Go-to-definition on a segment
jumps to that property's declaration. A misspelled segment is underlined alone, not the whole string,
with the unknown-member diagnostic the checker gives it. All of it reads the roster the checker records
for each class a key or path names, never a second walk of the class.

- **Memory:** one list of names and types per class a path reaches, the list `property<T>` already
  keeps with the checker's table for the length of the analysis. Nothing at run time.

**Declined:**
- **`{` as a trigger answered in every string.** A brace is plain text in most strings, and a list
  opening there would be a list nobody asked for.
- **Offering a whole path, `customer.address.city`, as one item.** A class with a relation back to
  itself has endless paths, and listing one class at a time is what keeps a request bounded.
