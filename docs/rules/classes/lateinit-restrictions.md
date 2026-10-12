`lateinit` is admitted on a property of every type: a class or an interface, a scalar, a string, an
array and a nullable `?T`. Its "no value yet" state is a tag of its own
(`rule:classes/an-unwritten-property-read-throws`), so it never collides with a value of the declared
type: `0`, `false`, `""`, `[]` and `null` are all values a `lateinit` property holds once written. A
promoted constructor parameter is refused (`E0426`), because binding the parameter already is the
assignment and there is nothing left to defer.

`lateinit readonly` is refused as well (`E0427`). `readonly` promises "assigned exactly once, during
construction"; `lateinit` promises "assigned after construction, and freely reassignable". They are
opposite claims about one property rather than a composable pair.

- **Memory:** none. The tag already exists on every slot, and only a `lateinit` read pays the one
  extra load of it.

**Declined:**
- **`lateinit` on a class or an interface type alone.** A scalar would then need a placeholder such as
  `= 0`, which reads as a real value and cannot be told apart from one, while the tag test costs the
  same on every type.
- **Refusing `?T lateinit`.** A `null` that is a decided answer, such as "this post has no author",
  still needs a separate state for "not looked up yet", and `isinit` tells the two apart
  (`rule:classes/isinit-reads-nothing`).
