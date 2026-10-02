The synthesis is a single function in `nvs-types` — a sibling of `check_array_literal`, not a change to
it — that types an array literal from its elements: `array<T>` when every element has the one type `T`,
and otherwise the reason it has none — two or more types with their canonical union, an empty literal,
or an element that could not be typed. It lives in the checker's crate rather than the editor's because
putting it beside the editor would mean a second implementation of union canonicalisation and literal
widening outside the type system that owns both, and the two would drift. The type table is the one home
for what a type is; the editor asks it a question.

**One compile path calls it, and only for a literal of one type.** `var`'s arm, for a bare literal
initializer, and the `foreach` subject under a `var` value binding are its two callers
(`rule:types/var-inference`). Everywhere else `check_array_literal` still returns `array<mixed>` where it
has no expectation, so a literal written against a type stays checked against it (`rule:types/arrays`),
and every other no-expectation position — an `echo` argument, a bare expression statement — types as it
did. A guard test counts the callers, so "no other position changed" is checkable rather than argued.

**A union is never attached to a binding.** Where the elements have two types the function returns the
union, and `var` refuses the literal and prints that union in its help line, for a person to write. The
editor action that narrows an annotation offers the same union as text in the file, read in a diff and
approved by a person (`rule:ide/narrow-an-annotation-to-its-literal`). A type only `var` held would be
visible nowhere, which is how a program acquires a type nobody chose. A literal of one type has no such
choice in it: the type is the one its elements state, so `var` takes it.
