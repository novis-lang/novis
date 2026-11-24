Writes text inside the open element as a CDATA section, such as `<![CDATA[a < b]]>`.

Inside a CDATA section, `<` and `&` are ordinary characters. The writer writes your text unchanged,
and it does not escape anything. This is useful for text that contains a lot of markup or code, such
as HTML in a news feed. A parser reads a CDATA section as normal text, so it gets back the same text
that `writeText` would give.

A CDATA section ends at `]]>`, so the text cannot contain `]]>`. If it does, `writeCdata` throws a
`LogicError` and writes nothing. Use `writeText` for such a text. A CDATA section must be inside an
element, so `writeCdata` also throws a `LogicError` when no element is open.

**The examples below** compare `writeCdata` with `writeText`, show the two errors, and put HTML into a
news feed.
