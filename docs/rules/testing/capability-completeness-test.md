Every `Core` class is on exactly one of two lists, and a test fails on a class that is on neither or
on both. On the first, the capability table, **every member of the class owes a row**: the
capability it needs, or *no capability* for a member that manipulates a string and touches no disk.
A member whose door asks a different capability by its argument, such as `Core\IO::open` by its
mode, has one row for each, and a row with no capability is a member's only row. On the second,
the list of classes that need nothing, the class has one row with a reason that is not empty, and
none of its members reaches a door. A class whose members differ is never on the second list: each
member gets its own row on the first.

A second test holds every door to its member's rows: each call to a door whose member is written on
the call, as a literal or a constant, asks only capabilities that member's rows name. The checker
reads the rows to hold a call to its namespace's grants
(`rule:security/authority-is-the-enclosing-namespace`), so a door asking more than its rows would
compile in a namespace never granted it. The same scan holds the second list: a door call naming a
member of a class on it fails, and so does a door call with a forwarded member in a module whose
every class is on it. A door call whose label is not a `Core` member is a language construct's,
such as `spawn script`, and it fails unless the construct table names every capability it asks.

There is no allowlist, and that is the point. A class or member that is hard to classify is one
whose capability has not been thought about, and the answer is to think about it, not to exempt it.
The one move this design forbids is the one an exception list makes cheapest. The claim is about
the whole *set* of classes, and a set with a growable exception list makes no claim at all. A
reason for none costs what declaring a real capability costs, is reviewed in the same file, and is
checked against the doors rather than trusted. No row on either list grants anything.

A third completeness test covers the other half: nothing in the standard library reaches the
operating system except through a door. Neither subsumes the other. One catches a member that goes
through a door undeclared, the other a member that reaches the OS with no door at all.
