Every member declaration in a class, interface or anonymous-class body writes exactly one of `public`,
`protected` or `private` — properties, class constants and methods alike, including a property carrying
only `readonly` or `lateinit`, and including an interface method, where `public` and `private` mean two
genuinely different constructs. An omission is a hard compile error whose fix hint offers `public`, so
accepting it on ported code preserves behaviour.

**There is no default, because there is nothing to default.** The question "what does an omission mean" has
no answer, and that is the point: nothing in the compiler, the reflection surface or a reader's head has to
hold one. The default an omission would need is a *security* default rather than a style one — an inferred
`public` is the mechanism by which an internal helper becomes public API because someone forgot a word, and
the author who forgot it is exactly the author who did not decide.

An enum body has no member slot at all (`rule:enums/no-class-machinery`), so nothing here reaches one, and
a property hook has no visibility slot either. What each level *means* at an access site is a separate
question, enforced separately; until that lands everywhere, a written `private` is an accurate declaration,
which is strictly better than an unwritten one. The formatter never inserts the keyword, because a formatter
that changes meaning is not a formatter.
