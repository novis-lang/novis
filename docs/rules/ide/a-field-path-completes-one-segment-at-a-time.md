Inside a string literal at a parameter typed `propertyPath<T>`, and inside a hole of a
`string<K>` literal, the completion list offers `T`'s public declared properties with their types, and
each `.` after a valid prefix offers the properties of the class that prefix reaches. Depth is
unlimited, and a path that loops back to a class it already passed costs nothing extra, because one
class's roster is listed per request.

A hole is the text after the last `{` before the cursor that no `}` has closed, read the way the checker
reads it: `{{` and `}}` are one brace and open nothing. A hole at a `string<property<T>>` offers `T`'s
names, and one whose `K` lists string values offers those values. Which characters ask for these lists
is `rule:ide/completion-is-asked-where-a-spelling-ends`'s, and which values may be offered at all is
`rule:ide/completion-offers-only-what-the-compiler-derived`'s; this rule owns the walk one segment at a
time, and hover, go-to-definition and the underline on a segment, which neither of those covers.

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
