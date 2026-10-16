Only the first character's case is checked. The rest of an identifier need only be alphanumeric ASCII, so a
run of consecutive capitals is accepted anywhere after the first: `HTTPClient`, `IOStream`,
`parseXMLPayload` and `HTTPXMLParser` all pass, and so do `HttpClient`, `IoStream` and `parseXmlPayload`.

An earlier rule required acronyms written as one word. It was revoked because deciding whether `HTTP` *is*
an acronym needs a maintained dictionary the compiler would have to ship and keep current, and even with
one it has no answer for two adjacent acronyms. Checking a single character needs neither a dictionary nor
a judgement, which is what makes the diagnostic's suggested rename always correct.

The cost is that a codebase can still be internally inconsistent about acronyms. That is accepted: the rule
exists to make a name's *category* legible from its spelling, and the leading character carries all of it.
