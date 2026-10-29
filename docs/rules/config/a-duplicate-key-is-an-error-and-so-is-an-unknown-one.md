The same key set twice in one file is refused (`E0604`), and so is a key the registry does not know
(`E0601`) — both name the file and the line. In a root-owned file where one table grants
capabilities, a line silently overridden by a later copy of itself is a security-relevant failure that
costs nothing to refuse, and a typo'd `capabilties` must fail at boot rather than read as "granted
nothing". A typo'd block header is refused the same way and claims no block. Both are `serde`'s own
behaviour with `deny_unknown_fields`; neither is new machinery.

**One exception: an unknown header with nothing under it is a warning (`W1024`), not `E0601`.** A
plain `[table]` header the typed tree has no block for, with no key under it at any depth, is removed
from its file before the file is typed, and the warning names the header, the file and the line. An
empty header sets nothing, so letting it through loses none of what the refusal protects: there is no
value to misread as "granted nothing". What it buys is that a file written for a release that had the
block, with every key commented out, still starts after the block is folded into another or removed.
Anything under the header — a key, a sub-table with a key, `name = {}` — keeps it `E0601`, and so does
an empty `[[entry]]` of an unknown array, because writing an entry states that it exists. An empty
header the tree *does* know is no block at all, the same as a file that never wrote it, and raises no
warning. `nvs_config::file::parse_with_warnings` is the one place this runs; it costs nothing on a
file that types first time.

**The four open blocks are not an exception to the refusal.** `[settings]`, `[secrets]`,
`[app.settings]` and `[app.secrets]` take any key that is a plain name, because the operator names
it (`rule:config/settings-and-secrets-are-a-programs-own-values`). What a closed block's field set
refuses, their own per-file check refuses instead, with the line: a key that is not a plain name, a
date or time, a secret that is not a string. A key no program reads is `nvs config check`'s warning
and not a refusal, because a program the tree does not name may read it.

On a reload the same diagnostic refuses the swap and the previous snapshot keeps serving, so a typo is
never published to a running server either.

**Both refusals are per file.** Across an `[[include]]` the same key set twice is not a duplicate but
an override, which `rule:config/later-wins-and-every-override-is-recorded` allows on condition that it
is reported with both origins. The property protected is that no assignment is *silently* shadowed;
inside one file the only way to hold it is to refuse, and an included file is refused inside exactly as
the root is.
