A `Core` member that removes `tainted` answers its sink's **carrier type** rather than the plain type
when **both** hold: its sink launders automatically — a value reaches that sink and is transformed
with no call written at the site — and its transform is **not idempotent**, so applying it to its own
output changes the output. Otherwise it answers the plain type.

The two conditions are one question asked twice: *a second application the source does not show, of a
transform a second application changes.* Only HTML output meets both today, which is why the HTML
escape answers a carrier and every other launderer on the roster — identifier quoting, URI component
and form-value encoding, regex quoting, terminal escaping — answers a `string`, by the predicate
rather than by exemption. Their results are legitimately concatenated into a larger string, and a
carrier would force a builder API onto four classes to close a hazard whose second call is already
visible in the source.

A launderer written for a *new* sink is measured against the two conditions, not against today's
table, and a sink that acquires an automatic launder reclassifies its own member the day it does.

**Why a `string` is the wrong answer at such a sink.** A `string` carries one bit where the domain has
three states: untrusted, plain text nobody has escaped, and already escaped for this sink. The second
and third are the same type, so an escaped value concatenated into tainted text is re-poisoned,
correctly, and the sink then escapes it a second time: `&` becomes `&amp;amp;` with no diagnostic
anywhere. With a carrier the three states are three types — `tainted string`, `string` and
`Core\Html\Markup` — and no operator moves a value backwards between them, so every ordering of a
concatenation is either correct or refused. Neither condition alone is a hazard: a non-idempotent
transform at a sink that never launders on its own takes a visible second call to break, and a
terminal escape is idempotent because its substitute contains no escape byte.

A registry guard holds it: the whole roster of launderers is listed with whether each answers a
carrier, so a launderer added in any class fails the guard until somebody places it against the
predicate.

**Memory:** nothing a program did not already spend. The sink's escape-and-lift makes a `Markup`
anyway, so a carrier answer removes the second escape rather than adding a wrapper.

**Declined:**
- **The HTML sink detects already-escaped text and skips it.** `&amp;` in the input is text that said
  `&amp;`, and it must come back as `&amp;amp;`; a sink that guesses is one an attacker can aim
  (`rule:errors/ambiguous-input-refused`).
- **A third qualifier, `escaped string`.** New grammar, a new keyword and a third axis in the checker,
  to say what a carrier class already says for free.
- **A carrier for every non-idempotent launderer**, such as a URI component, a database identifier and
  a regex pattern type. It forces a builder API onto four classes whose values are normally assembled
  by concatenation, against a hazard that takes a visible second call to reach.
