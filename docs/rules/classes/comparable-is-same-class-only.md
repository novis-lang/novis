`compareTo(self $other)` fixes the other operand to the implementing class. Comparing two objects
that are not both known to be that same class is a diagnostic even when both classes implement
`Comparable` independently — there is no cross-class overload and no implicit widening from a
subclass's `compareTo` to an ancestor's.

The alternative is a resolution question with no non-arbitrary answer: which of two classes' methods
decides the order of a mixed pair, and what a program should conclude when they disagree. Fixing the
parameter to `self` removes the question instead of answering it.

A type that genuinely needs to be ordered against a different type says so with an ordinary named
method — `Money::isGreaterThan(Distance $d): bool` reads oddly on purpose. The cost is real: no ordering
crosses two classes until a parameterized `Comparable<T>` is designed, and no such generic exists
yet.
