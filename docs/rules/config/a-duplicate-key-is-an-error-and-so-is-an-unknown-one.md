The same key set twice in one file is refused (`E0604`), and so is a key the registry does not know
(`E0601`) — both name the file and the line. In a root-owned file where one table grants
capabilities, a line silently overridden by a later copy of itself is a security-relevant failure that
costs nothing to refuse, and a typo'd `capabilties` must fail at boot rather than read as "granted
nothing". A typo'd block header is refused the same way and claims no block. Both are `serde`'s own
behaviour with `deny_unknown_fields`; neither is new machinery.

On a reload the same diagnostic refuses the swap and the previous snapshot keeps serving, so a typo is
never published to a running server either.

**Both refusals are per file.** Across an `[[include]]` the same key set twice is not a duplicate but
an override, which `rule:config/later-wins-and-every-override-is-recorded` allows on condition that it
is reported with both origins. The property protected is that no assignment is *silently* shadowed;
inside one file the only way to hold it is to refuse, and an included file is refused inside exactly as
the root is.
