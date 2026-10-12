A property declared `lateinit` is exempt from the constructor-must-assign obligation, for the pattern
that obligation cannot express: a container, an ORM, or any setter injection that populates a property
after `new` returns. It adds no new type and no new value: a `lateinit` property holds exactly the
values its declared type holds, and reading it before its first write throws the error
`rule:classes/an-unwritten-property-read-throws` defines. `isinit` tests the state without reading
the property (`rule:classes/isinit-reads-nothing`).

Writing it, the first time or any later time, is an ordinary property write. There is no write-once
tracking: once assigned it is indistinguishable from a property the constructor set.

What it spends is a second way a non-nullable read can fail at run time, in code that never touches
reflection. The alternative was making every injected dependency `?T` forever and null-checking it at
every use, which is an ergonomic wall rather than a guarantee. Nothing else changes: a copy of an
object whose `lateinit` slot is still unwritten inherits the same unwritten slot and throws under the
same rule.

- **Memory:** none. The unwritten state is a spare discriminant of the tagged value every property
  already has, so a `lateinit` property costs no byte an ordinary property does not.

**Declined:**
- **Leaving deferred assignment to reflection alone.** Ordinary setter injection would have to build
  its objects through reflection just to get an initialization order the language can express
  directly.
- **A write-once variant, assignable once at any time and then frozen.** It needs a tracked write
  count that a freely reassignable property does not, and `lateinit readonly` is refused
  (`rule:classes/lateinit-restrictions`).
