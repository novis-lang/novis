The token the trusted walk lands on resolves one of three ways.

A token carrying the port the hop connected from — `203.0.113.9:54321`, or `[2001:db8::1]:443`, the one spelling in which a v6 address can — has the port **removed** and the address read, before the trust test as well as after it. Azure's front ends and IIS write these, a port names no different address, and this is the same *accept verbatim* branch as a `Host` port. A port that is not a number is not one, and the token falls to the refusal below.

A token that **withholds** the address — `unknown`, or an obfuscated `_hidden` per RFC 7239 — makes `clientIp` **`null`**: the hop said there is no address to report, which is a fact and not a defect, and Squid emits `unknown` with `forwarded_for` off. Only a *trusted* hop can put one where the walk lands; a client's own sits left of the address its proxy appended, and is never reached.

A token that neither names an address nor withholds one is a **`400`**: the value was about to be used and cannot be read. That row, and a request path containing a dot-segment or an encoded separator, are the two the server adds to `rule:errors/ambiguous-input-refused`'s closed list.
