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

**On disk in the checker.** `nvs_ext::manifest` reads `sink` and `source` as the only qualifier keys,
refuses any other key, and refuses a `secret` or `tainted` written into a type
(`crates/nvs-ext/tests/load.rs`). `nvs_types::ext_lib` writes the two declarations into the
`MethodSig` fields a `Core` row fills, and the call is checked by the code a `Core` call goes through
(`crates/nvs-types/tests/extensions.rs`).
