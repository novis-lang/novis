Capturing output answers **the carrier of the sink in force**, not a plain `string`. The reason is not
symmetry: the captured bytes have *already* been through the sink, so handing them back as a `string`
and re-emitting them would escape them a second time and corrupt the page.

The same rule gives a spawned isolate's captured result its type
(`rule:security/isolate-output-is-captured`), by the same argument and with the same fix. Inheriting
rather than capturing needs no rule at all: the child's carrier is the parent's, so appending composes
the way carrier addition already does.

**A body member's bytes pass the sink too, where they become a carrier.** `Core\Response::text`,
`::json`, `::bytes` and `::stream` write verbatim, because what they write is a response with its own
media type. Inside a capture, and in a `spawn script` child, it is not: it becomes the carrier, or is
appended to the parent's stream. There those four write as `echo` writes a `string`, escaped under the
HTML sink and substituted under every other, so a child's `text("<script>…")` shows as text in the
parent's page and an `echo`ed one is escaped once. Bytes that are not UTF-8 are rendered byte by
byte and still make a capture throw. `html` writes a `Markup` and `sendFile` a file the program
named, so both stay verbatim. The cost is one copy of such a body where the rendering changes it.
(`crates/nvs-runtime/src/ctx/output.rs`'s `a_body_that_becomes_markup_is_escaped_and_a_response_is_verbatim`,
`tests/hostile/lang/concurrency/isolates-spawn-script-and-await/02-a-child-writes-markup-the-parent-prints.nvs`.)

This is the third instance of one rule rather than three members that each argued it alone — the other
two being a laundering escape and an approved URL — and stating it once is what keeps a future
capturing member from answering the wrong type (`rule:security/launderer-answers-a-carrier`).
