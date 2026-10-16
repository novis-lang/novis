A symmetric operation gets a symmetric name: `encode`/`decode`, `split`/`join`, `pack`/`unpack`,
`escape`/`unescape`, `trimStart`/`trimEnd`, `startsWith`/`endsWith`, `indexOf`/`lastIndexOf`,
`first`/`last`. If one half exists, the other half's spelling is decided by rule rather than chosen.

Which *pair* to reach for is also a rule: `encode`/`decode` when the other side is a machine format —
JSON, serialization, base64 — and `parse`/`format` when a human writes or reads it, as with time, CSV and
URIs. That removes a per-member judgement call that would otherwise go differently every time, and it means a reader who
has found one half knows the other's name without looking.
