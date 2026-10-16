The URL parameter of an HTTP client's request members, and the address parameter of a raw connect, are
unqualified: a `tainted` value at either position is a compile-time diagnostic, exactly as at a query
text parameter. Server-side request forgery starts with an attacker-chosen address, and this is the
position where that is visible.

The plain form covers URLs the program itself authored — a literal, a configuration value, a composed
path — and a literal is additionally validated while checking
(`rule:expressions/intrinsic-constant-arguments`). A URL that genuinely came from outside goes through the
laundering member that also **pins** the address it resolved to, so the check and the connection
cannot disagree about which host was approved.

Refusing at the call site is only half of it: the address is judged again at run time against the
capability's own policy (`rule:security/net-address-policy`), because a hardcoded hostname can still
resolve into a private range.

Request forgery is an injection, but it is the one that cannot be settled at compile time alone: a
well-formed URL is harmless or not according to the address it resolves to, which is known only at
connect time and can differ between two resolutions of one name. Nor can it be settled at run time
alone, since nothing in the source would show a developer which URL is attacker-influenced. So it gets
both layers. Fetching a URL somebody supplied is a legitimate feature — webhooks, previews, imports — and
stays two lines, the second of which is the launderer that records the decision.

**Declined:**
- **The run-time policy alone, with no `tainted` refusal.** The developer gets no signal at `nvs check`
  and the failure appears in production, and this class alone would lose the compile-time audit every
  other injection has.
- **The compile-time refusal alone, with a launderer checking the URL's shape.** `http://169.254.169.254/`
  is well-formed, so the attack passes straight through.
- **A launderer answering a plain `string`.** It reopens the gap between the check and the connection
  (`rule:http-server/allow-url-pins-the-address`).
- **Leaving it to the application, with documentation.** Every application re-implements it, most
  incompletely, and a language that makes SQL and HTML injection compile errors would be treating this one
  as a matter of care.
