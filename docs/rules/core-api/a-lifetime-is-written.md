A signature's lifetime is a **required key holding a nullable value**: both a written instant and a written
`null` are spellings, and omitting the key does not compile. `null` is the forever spelling.

An omission is not a default. A permanent signed link is a permanent bearer credential — written into
browser history, `Referer` headers, proxy logs and chat unfurls, and it never stops being one — so
"forever" should be something a person typed rather than something a missing key chose.

This is the one place a required key deliberately holds a nullable value, and it reads differently from
`rule:core-api/a-written-null-removes`: there `null` clears an optional field, here it selects the
unbounded lifetime on a field that must be written either way.

**Declined:**
- **Mandatory expiry.** It is JWT's rule (`rule:security/jwt-expiry-is-mandatory`), and its reasoning is
  about what a JWT is for; the signed cookie carries no lifetime at all. A mandatory lifetime also meets a
  page left open all day by pushing applications toward a long `until` written once and never revisited,
  which is the worse outcome. A written `null` gets the deliberation without that workaround.
