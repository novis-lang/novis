```nvs
function Core\Program::id(): string;
```

`BLAKE3(unit content hashes in program order ‖ env_hash)`, rendered as all 32 bytes in lowercase hex —
64 characters, never truncated by the runtime, because a caller that wants eight of them can take eight
and one that wants all 32 cannot get them back. Program order rather than sorted order: a graph whose
units resolve in a different order is a different program, and an identity that cannot see that is not
one.

It is the one member of its class with a body, and that is circularity rather than preference. Folding
it into a constant would write the id into a unit as a literal, which changes that unit's bytes, hence
its content hash, hence the id just folded. The host computes it once, where the resolved graph and the
environment digest are both in hand, writes it onto the context before any Novis code runs, and the
member reads that string back.

**Computed at program resolution and at the hot-reload pointer swap, never per call and never lazily.**
Those are the only two moments a running host's set of units changes; a lazy first-call compute would
put a hash of every unit digest on one unlucky request's path. It holds 32 bytes per program and one
BLAKE3 combine per resolution (`rule:programs/memory-priority`).

Plain `string`, never `secret`: every use of the id is an echo — a cache-busting URL segment, a
response header, the field that tells one deployment's log lines from another's — and `secret` refuses
an echo by design. A context nobody wrote an id onto makes the member **throw**, rather than answer an
empty string or invent one, because callers key caches and invalidate CDNs on this value and a wrong
identity is worse than no answer.
