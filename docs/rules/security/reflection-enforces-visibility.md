Reading **metadata** — a member's existence, name, declared type, visibility, attributes, doc comment
— is always available, because that is introspection of the program's *shape*. **Acting on** a member
is different: calling a reflected method, or reading or writing a reflected property, runs through
exactly the visibility check, and any declared property observer, that ordinary code at that call site
would face. A reflective call from outside a class to one of its `private` methods fails the way an
ordinary out-of-class call would. A `protected` member is where that promise is worth stating twice:
a reflective read, write or call written inside a subclass's body reaches it, because an ordinary
access there reaches it, while the same site is refused the class's `private` members. A `private`
member is reached by name only from the class that declares it, on an instance of that class or of
any subclass. The declaring class is the one whose descriptor has that very member while no class it
extends or implements has it: the same slot for a property, the same code address for a method, the
same name for a constant. So a subclass's body is refused its parent's `private` property, method,
constant and constructor, and the parent's body reaches them on a subclass's instance. A method a
subclass declares again has its own code, so it belongs to the subclass. A `private` constant a
subclass declares again over its parent's is refused to that subclass by name, since a constant row
carries no declaring class; this errs toward refusing. Constructing
through reflection calls the constructor under the same check, so a `private` constructor is reached
from its own class's bodies and from nowhere else.

**There is no `setAccessible(true)` and no equivalent.** It is rejected outright rather than left
undocumented, because an escape hatch for reaching a private member from anywhere is a structural
privilege-escalation path, and security does not get spent on convenience. The cost is that a
serializer or a test helper that reached into private state through reflection has no port: it needs
the declaring class to offer the access, which is the same answer `rule:testing/private-in-the-same-file`
gives a test.
