An input is **ambiguous** when two conforming readers can disagree about what it says. An input is
merely **unusual** when every reader agrees and the spelling is only uncommon.

- **Ambiguous → refuse the whole message or operation**, with a diagnostic naming the defect. Never
  a partial read, never a repair.
- **Unusual → accept verbatim.** No renaming, no case folding, no truncation, no substitution.

The ambiguity list is **closed and written down**: `rule:errors/http-message-defects`,
`rule:errors/cookie-name-bytes`, `rule:errors/multipart-part-count` and
`rule:errors/path-component-refusals` are all of it. A defect not on the list is accepted; adding
one is a change to this rule. That is what makes the rule testable and fuzzable, and what stops
"strict" from growing every time somebody reads a specification more carefully.

**"A proxy does it earlier and better" is true for limits and false for parsing.** Request smuggling
*is* a parser differential — it exists precisely because two parsers read one byte stream
differently — so delegating leniency to the proxy is not a mitigation, it is the mechanism. A
production deployment always has a proxy in front of it, which is exactly the deployment a lenient
origin parser is dangerous in.

Strict parsing is not slower. Validation folds into the pass that already touches every byte to find
delimiters; it is *lenient* parsing that needs extra work to unfold, re-scan and normalise.

The unusual half is as much of the rule as the refusals: a header name with an underscore, or an
unknown but well-formed one, is accepted verbatim.

**Declined:**

- **Normalise and continue.** It gives the most interoperability, and it is the posture behind a run of
  parser vulnerabilities in other runtimes: a folded header line unfolded and parsed, a header line with
  no colon treated as a header, a long redirect target truncated and followed, a hostname with NUL
  accepted. It also guarantees that Novis and a proxy in front of it can disagree about what a request
  says.
- **Total strictness**, refusing anything that does not match the HTTP grammar exactly. It is simpler to
  state, and it refuses requests every other origin accepts (underscores in header names are common). It
  also makes the list open-ended, which the closed list exists to prevent.
