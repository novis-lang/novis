`Core\Reflect` is read-only structural introspection, and it is a first-class feature rather than an
extension a deployment might not have compiled in. It reaches classes, interfaces, enums, methods,
properties, constants, parameters and attributes, from a value or from a class name, and it reports
which methods an interface declares as part of its contract versus as an internal helper. Traits are
absent because they do not exist.

Reflective access **enforces the same checks ordinary code would**: there is no
`setAccessible(true)`, so a private property is not readable through this door either, and a
reflective write runs the property observer an ordinary write would run. The refusal is
distinguishable from a misspelling, which is what makes the answer useful rather than merely safe.

What it costs is that a serializer or a container cannot reach state its author did not expose. That
is the trade: the alternative is a reflection API that makes every access modifier in the language
advisory.
