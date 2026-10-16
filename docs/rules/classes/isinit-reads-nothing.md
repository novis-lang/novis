`isinit($obj->prop)` is a construct, parsed like `isset`, whose value is a `bool`: `true` once the
`lateinit` property has been written, `false` before. It does not read the property. No `get` hook
runs, no `onPropertyGet` is told, and nothing throws: it compiles to the receiver, one load of the
slot's tag and one compare (`rule:classes/an-unwritten-property-read-throws`). A written `null` is a
value, so `isinit` is `true` after `$p->author = null`. On a hooked `lateinit` property it tests the
backing slot.

It is a compile error (`E0853`) on a property that is not `lateinit`, which always has a value, and on
an operand that is not a `->` property read. It is a compile error (`E0854`) on a receiver whose class
is not known at compile time, such as `object` or a shape, because the slot is named from the class;
an `is` test first narrows the receiver to one.

It is a construct and not a method because a method call evaluates its argument, and reading the
property is the one thing that throws here.

- **Memory:** none. It allocates nothing and holds nothing.

**Declined:**
- **A `Core\Reflect` method taking the property's name.** A misspelt name would be a runtime error,
  and the test would no longer compile to one tag compare.
- **Letting `isset` answer it.** `isset($x)` is `$x != null`
  (`rule:classes/unset-is-refused-on-a-property`), and on a `?T lateinit` "written as `null`" and "not
  written yet" need different answers.
