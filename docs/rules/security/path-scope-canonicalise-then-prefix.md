A path-bearing capability resolves by one comparison: **canonicalise both sides, then compare whole
components.** A traversal through `..` does not reach a root it was not already under, a symlink
planted under a granted root does not carry the root's grant to its target, and a sibling directory
whose name merely starts with the root's is not inside it. There is one implementation of this rule in
the tree, and adding a second is how one of the callers ends up accepting a symlink.

The granted roots are canonicalised **once, when the snapshot is built**: a root still spelled the way
the operator typed it is a comparison against the wrong thing, and doing it per call would put a
resolution on the grant side of every check.

A write to a file that does not exist yet cannot be canonicalised, and creating it to find out whether
creating it is allowed is obviously wrong. So the argument canonicalises its **deepest existing
ancestor** and re-appends the remainder, with the remainder refused outright if it contains `..` — the
one component that could still escape after the ancestor is pinned.

A granted root that does not exist yet — a socket nothing has bound — is resolved by that same walk,
so both sides are spelled by one resolution. A root left as typed could not match a canonical
argument wherever a directory above it is a symlink, as `/tmp` and `/var` are on macOS.
