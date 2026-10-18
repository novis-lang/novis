A nullable type `?T` is the type `T` or `null`. A variable can be `null` only when its type allows
it. A union `A|B` is either of the two types. Test which one a value is with `is` before you use it
as one of them.

A single-value type is one value written as a type, such as `"asc"` or `3`. A union of them is a
list of allowed values. A setting that may only be one of three words has a type that names those
three words. Any other word is not accepted. An enum case is a type in the same way, so a function
can accept two cases of an enum and not the third.

**The examples below** show a setting that may be missing, a sort order limited to three words, and
a handler that converts an untrusted request into the values it accepts. The last three show each
kind of type at once, and two compile errors.
