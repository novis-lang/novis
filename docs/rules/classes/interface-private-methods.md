An `interface` method declared `private` and carrying a body is an internal helper for that
interface's own method bodies. It is callable from them through `$this->`, and it is not part of the
contract: an implementor never defines it, can never override it, and a call from outside the
interface's own bodies is a diagnostic.

This is the one trait shape a public default method does not cover, since a default is always part of
the public surface. Without it, an interface with shared behaviour would have to publish every helper
that behaviour needs, which is a wider API than the author meant and a wider one every implementor
inherits.

The precedent is Java 9's private interface methods, and the addition is small: it changes who may
call a member, not what an interface may hold. Interfaces still declare no state.
