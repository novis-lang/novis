A property declared `lateinit` is exempt from the constructor-must-assign obligation, for the pattern
that obligation cannot express: a container, an ORM, or any setter injection that populates a property
after `new` returns. It adds no new type and no new value — a `lateinit` property is exactly as
non-nullable as any other, and reading it before its first write throws the error
`rule:classes/an-unwritten-property-read-throws` already defines, now reachable from ordinary code
rather than only through reflection.

Writing it, the first time or any later time, is an ordinary property write. There is no write-once
tracking: once assigned it is indistinguishable from a property the constructor set.

What it spends is a second way a non-nullable read can fail at run time, in code that never touches
reflection. The alternative was making every injected dependency `?T` forever and null-checking it at
every use, which is an ergonomic wall rather than a guarantee. Nothing else changes: a copy of an
object whose `lateinit` slot is still unwritten inherits the same unwritten slot and throws under the
same rule.
