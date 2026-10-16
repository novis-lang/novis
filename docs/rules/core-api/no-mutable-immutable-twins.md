Every `Core` value type is immutable, so there is no mutable twin to choose between. There is no
`DateTime`/`DateTimeImmutable` pair, and no member returns a mutable view of a value type.

A pair like that exists only where a library's original type was mutable and could not be removed; a
library written once has no such type to keep. Immutability also removes the question `rule:core-api/nothing-mutates`
would otherwise have to answer twice — a value type with no mutator has no in-place variant to argue about
— and it means a value handed to another request-scoped object cannot be changed underneath it.

The cost is that every derivation allocates a new value, which copy-on-write makes cheap for the array-
and string-shaped ones and genuinely a copy for the small structs, where it is a few words.
