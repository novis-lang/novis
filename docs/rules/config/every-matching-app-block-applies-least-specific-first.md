More than one `[[app]]` can match one entry file, and all of them do. They are applied in order of
**increasing specificity** — shortest `root` first, longest last, an `entry` match last of all — so
`/srv/www/shop/bin/import.nvs` takes `memory` from the `/srv/www/shop` block and `wall_time` from its
own, while inheriting everything neither states. A host-wide block and an application-specific one
compose instead of competing, and a narrow block never has to restate what the wide one said.

This is not a third precedence rule: it is `rule:config/later-wins-and-every-override-is-recorded`
ordered by specificity instead of by file position, run by the same merge, so a directive one block
takes from another — or from the global block — is reported with both origins without asking twice.
`rule:config/a-value-array-replaces-and-a-table-appends` holds inside a block too: a capability list
in a narrower block replaces the wider one's.

**Two blocks resolving to the same canonical path are a duplicate rather than a refinement, and are
refused** (`E0609`). It is the path and not the spelling, so two routes to one directory are caught
as well, and specificity has no order left to decide which of them wins with.

Matching happens once per entry file, at the point it is resolved — for a served request at
compile-time route resolution, never per request.
