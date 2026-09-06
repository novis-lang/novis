A signature's lifetime is a **required key holding a nullable value**: both a written instant and a written
`null` are spellings, and omitting the key does not compile. `null` is the forever spelling.

An omission is not a default. A permanent signed link is a permanent bearer credential — written into
browser history, `Referer` headers, proxy logs and chat unfurls, and it never stops being one — so
"forever" should be something a person typed rather than something a missing key chose. Making expiry
mandatory would be the other answer, and it is rejected here as belonging to a specific token format rather
than to the general operation.

This is the one place a required key deliberately holds a nullable value, and it reads differently from
`rule:core-api/a-written-null-removes`: there `null` clears an optional field, here it selects the
unbounded lifetime on a field that must be written either way.

**Designed, not shipped.**
