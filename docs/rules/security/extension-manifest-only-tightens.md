Every declaration an extension manifest may carry is a **restriction**: it either rejects a call that
would otherwise compile, or adds a qualifier the caller must discharge. Nothing a manifest can say
makes a calling program accept more.

This matters because a manifest is written by the extension's author, not by us. Hashes are pinned and
signatures verified, but the analysis must not *depend* on that being done correctly. Under this rule
it does not: a hostile manifest can make its own extension unusable, and cannot make a calling program
less safe than contagion alone would (`rule:security/extension-contagion`).

An extension call site is checked by the same code path as a `Core` call site, reading the qualifier
from the registered signature rather than from a table of built-ins. There is no second analysis and
no extension-specific relaxation, so the two cannot drift.

**Not on disk.** There is no extension tier in the tree — no component loader, no manifest reader, no
qualifier axis in a world file — so none of this is enforced today.
