Reading **metadata** — a member's existence, name, declared type, visibility, attributes, doc comment
— is always available, because that is introspection of the program's *shape*. **Acting on** a member
is different: calling a reflected method, or reading or writing a reflected property, runs through
exactly the visibility check, and any declared property observer, that ordinary code at that call site
would face. A reflective call from outside a class to one of its `private` methods fails the way an
ordinary out-of-class call would. A `protected` member is where that promise is worth stating twice:
a reflective read, write or call written inside a subclass's body reaches it, because an ordinary
access there reaches it, while the same site is refused the class's `private` members.

**There is no `setAccessible(true)` and no equivalent.** It is rejected outright rather than left
undocumented, because an escape hatch for reaching a private member from anywhere is a structural
privilege-escalation path, and priority 1 does not get spent on convenience. The cost is that a
serializer or a test helper that reached into private state through reflection has no port: it needs
the declaring class to offer the access, which is the same answer `rule:testing/private-in-the-same-file`
gives a test.
