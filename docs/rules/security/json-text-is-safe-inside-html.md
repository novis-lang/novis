Every JSON text Novis writes from a program's values has `<`, `>`, `&`, `'`, U+2028 and U+2029 written
as `<`, `>`, `&`, `'`, ` ` and ` ` in every string and every key. There is
no option to turn it off. The text stays valid JSON, and every decoder reads back the same value.

**Why these six.** A JSON text often ends up inside an HTML page: in a `<script>` element, in an
attribute, or in a JavaScript string. There a string with `</script>` closes the element and the rest
of the value runs as markup. `<` and `>` cannot open or close a tag once escaped. `&` cannot start an
entity, so an attribute's HTML decoding gives back the text the encoder wrote. `'` is there because a
page often puts JSON in a single-quoted attribute, such as `data-config='{"page": 2}'`, since JSON's own
`"` ends a double-quoted one. With `'` and `&` escaped, a value cannot end that attribute. U+2028 and
U+2029 end a line in JavaScript before ES2019, so inside a script string they end the statement.

**Escape, do not taint.** A program's own tainted value inside `Core\Json::encode` is safe in a JSON
response already (`rule:security/response-body-is-one-typed-member`), and that is the common, correct
case. Making `encode`'s result tainted when its input is would make every API that returns user data
add a launder step for nothing. Taint also cannot tell a JSON response from a page the text is pasted
into, and the escaped text is safe in both.

**One escape table, so every writer agrees.** The six are rows of the one table the encoder writes
every string with, beside JSON's own escapes, in the same pass over the string's bytes. It applies to
the compact and the pretty profile, to keys and values, and to each piece of `Core\Json::encodeEach`.
`{ escapeUnicode: true }` is that same pass with one more class of byte, the first byte of a non-ASCII
character, so it is no second pass over the output either. Every other
member that writes a program's values as JSON goes through that encoder: `Core\Response::json`,
`Core\Http\Client`'s `json` option, `Core\Sse`'s payload, `Core\Queue`'s job arguments and `Core\Jwt`'s
claims.

**What it spends.** Five bytes more for each escaped character, the apostrophe included, and nothing
for a string without them. The encoder reads every byte of a string once, through one table lookup,
so a byte that needs no escape costs that one lookup however many rows the table has.

**Three JSON writers are outside it, because none of their text reaches a page as JSON text.**

- A JWT's header and payload are base64url, so a page never reads their JSON as text. The header's
  `kid` and `typ` are written by `Core\Jwt` itself.
- A log line, `Core\Log`'s `fields` included, goes to the log's destination. The error page renders a record through the HTML
  renderer, which escapes every value for HTML.
- `nvs`'s own JSON output, such as `nvs check --json` and the debug UI's API, is read by tools. The
  debug UI serves its API as `application/json` with `nosniff` and a content security policy.

**Declined.**

- **An option to write the six characters as they are.** PHP has the escape as the flag `JSON_HEX_TAG`,
  off by default, which is the kind of option a program forgets. Go's encoder escapes by default, as
  this one does. A program that needs the raw bytes for a reader that compares text byte for byte is a
  later decision, made when such a reader is named.
- **A tainted result.** Above, under *Escape, do not taint*.
- **A second pass over the finished text.** It would cost one more copy of every document, and the
  writer already reads each string's bytes.
- **Leaving out the apostrophe,** as Go does. It is the most common of the six in text, but a
  single-quoted attribute is a common place for JSON, and five bytes per apostrophe is the price.
