The library parses exactly four grammars — a regex pattern, a format template, a date pattern, and a
byte-packing format — and every one of them is an instruction under `rule:security/sink-predicate`.
So each of the four is a sink, and the count that fixes the library's grammar surface doubles as the
roster.

**A regex replacement is a sink too.** It is not a grammar of its own under
`rule:core-api/no-mode-strings`, since it belongs to one member, but `Core\Regex::replace` reads `$1`,
`${name}` and `$$` in it, so a `tainted` one would let the request choose which captured text lands
where. Text from
outside goes into a result through `Core\Regex::replaceWith`, whose function returns it as text and
never reads it as a template.

**An XML processing instruction is a sink for the same reason, though it is written rather than
parsed.** `Core\Xml\Writer::instruction` escapes nothing, since XML has no escape inside one, and its
data is an instruction to the program that reads the document: an `xml-stylesheet` names the file a
browser fetches and runs. Its target and data refuse `tainted`; the writer's text, attribute, comment
and CDATA members escape or refuse what they cannot write, and stay launderers.

**No launderer is added for any of them, deliberately.** A grammar is written by the program, not
received by it, so the fix at a failing call site is to use a literal — which is folded at compile time
for all four (`rule:expressions/intrinsic-constant-arguments`) — or,
for a genuinely dynamic template drawn from a translation catalogue, `rule:security/assert-trusted` with
its written reason.

A regex quoting member remains the one exception on the roster, because a regex is the one grammar
that routinely needs a runtime value *inside* it rather than *as* it
(`rule:security/regex-pattern-is-a-sink`).

**Declined:**
- **A replacement option that reads the text literally.** `replaceWith` already inserts text as it is,
  and an option would be a second way to write one thing.
