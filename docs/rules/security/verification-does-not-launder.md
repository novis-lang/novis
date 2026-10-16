Claims returned from token verification are **`tainted`**. A signature proves origin, not safety for
any sink: the payload may be authored by a third-party issuer, and even a self-issued token routinely
carries user-supplied data. Treating verification as laundering would be exactly the false confidence
refused everywhere else (`rule:security/launderers-are-sink-named`).

Cookie payloads the application itself sealed are the one case where the value round-trips through our
own authenticated encryption unchanged, and they come back **unqualified** — sealing is an
authenticated operation over a value that was already plain when it went in. The asymmetry between the
two is the rule, not an inconsistency: one is a value we had, the other is a value we were handed.
