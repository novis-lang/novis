- **`$x is Iterable` with no type argument is `E0442`, so an agreement row over `iterable`'s members
  cannot be spelled the obvious way.** `rule:iteration/concrete-generic-implements` makes the argument
  mandatory at every site naming either reserved interface, an `is` test included, and the diagnostic
  says so where a reader expects a bare interface name to work. Write `$x is Iterable<int> || $x is
  Iterator<int>`: the walk compares the label alone, so which argument is written never changes the
  answer. [until: reviewed 2026-09-14]
