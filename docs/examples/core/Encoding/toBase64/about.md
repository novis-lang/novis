Writes bytes as base64 text, so that they can travel where only text may go.

Base64 writes any bytes using the letters `A` to `Z` in both cases, the digits `0` to `9`, and the
two symbols `+` and `/`. Every three bytes become four characters, so the text is a third longer
than the buffer. When the last group is short, `=` is written to fill it. Every byte has a
spelling, so this member never fails, and the empty buffer gives the empty string.

Programs use this form wherever only text may travel: an HTTP header, an email attachment, a small
image inside a stylesheet, a field in a JSON document.

**Good to know:** `+`, `/` and `=` all have their own meaning inside a URL. For a URL or a JSON Web
Token use `Core\Encoding::toBase64Url`, which writes `-` and `_` in their place and no padding at
all.
