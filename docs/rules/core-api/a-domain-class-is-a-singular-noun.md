A domain class is named with a singular noun, and the object types it constructs nest under it:
`Core\Time`, `Core\Time\Instant`, `Core\Time\Duration`. Never `Core\Times`, never `Core\TimeUtils`, never
`Core\TimeHelper`.

The plural and the `Utils`/`Helper` suffixes are the two ways a static-method class announces that nobody
decided what it was for, and both invite a second class beside the first. A singular noun names a domain,
and nesting puts the domain's own types where a reader looking at the class already is. It also composes
with the reserved namespace (`rule:core-api/reserved-namespace`), since a nested name is reserved by the
same rule as its parent.
