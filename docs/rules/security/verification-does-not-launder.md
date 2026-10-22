Claims returned from token verification are **`tainted`**. A signature proves origin, not safety for
any sink: the payload may be authored by a third-party issuer, and even a self-issued token routinely
carries user-supplied data. Treating verification as laundering would be exactly the false confidence
refused everywhere else (`rule:security/launderers-are-sink-named`).

A cookie the application itself sealed is no exception: `Core\SignedCookie::open` answers `tainted
string` too. `seal` accepts a `tainted` value, so the payload may be a visitor's text that a later
request reads back, which is stored injection if it comes back unmarked. A program that knows one
stored value is safe for one sink says so at that sink, with its launderer.

**Declined:**
- **An unqualified cookie payload, on the ground that the program sealed it.** The tag says which
  program sealed the bytes and not where they came from. `seal` would have to refuse every `tainted`
  value for the round trip to be safe, and a cookie that remembers a visitor's choice is the common
  case.
