An include tree that reaches a file already on the chain is refused with the whole chain named
(`E0606`), and nesting deeper than eight files is refused as depth under the same code — not as a
cycle, because it may not be one. An include tree eight files deep is one no operator can read whether
or not it ever closes.

A cycle is caught by **name, not by depth**: the trust check hands back the canonical path, so a ring
built out of symlinks closes on a file the resolver has already seen even though no two spellings on
the chain match. A lexical comparison would let that ring through until the depth cap caught it, and
report the wrong thing.
