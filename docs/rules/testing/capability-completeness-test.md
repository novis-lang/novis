A class is **capability-bearing** when any of its members has a row in the capability table. For
such a class **every member owes a row**, and a member that genuinely needs no
capability — one that manipulates a string and touches no disk — declares that by entering the table
with *no capability*, rather than by staying out of it. A member whose door asks a different
capability by its argument, such as `Core\IO::open` by its mode, has one row for each, and a row with
no capability is a member's only row.

A second test holds every door to its member's rows: each call to a door whose member is written on
the call, as a literal or a constant, asks only capabilities that member's rows name. The checker
reads the rows to hold a call to its namespace's grants
(`rule:security/authority-is-the-enclosing-namespace`), so a door asking more than its rows would
compile in a namespace never granted it.

There is no allowlist, and that is the point. A member that is hard to classify is a member whose
capability has not been thought about, and the answer is to think about it, not to exempt it — so
the one move this design forbids is the one an exception list makes cheapest. The claim is about the
whole *set*, and a set with a growable exception list makes no claim at all. Declaring "nothing"
costs what declaring a real capability costs, is reviewed in the same table beside its reason, and
grants nothing, because no row grants anything.

A second completeness test covers the other half: nothing in the standard library reaches the operating
system except through a door. Neither test subsumes the other — one catches a member that goes
through a door undeclared, the other a member that reaches the OS with no door at all.
