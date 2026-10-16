A nullable type holds its type or nothing. Nothing is a type of its own, so an ordinary number can
never quietly be empty. A union holds either of the types it names, and a program asks which one is
there before treating it as one of them.

A single-value type is one value used as a type, such as `"asc"`. A union of them is a set of
allowed values. A setting that may only be one of three words has a type that names those three
words, and a fourth word is not accepted where it arrives. An enum case is a type in the same way,
so a function can accept two of an enum's cases and not the third.

**In plain words:** the type lists what is allowed, and nothing else gets in. The examples show a
setting that may be missing, a sort order limited to three words, and a handler that turns an
untrusted request into the values it will act on.
