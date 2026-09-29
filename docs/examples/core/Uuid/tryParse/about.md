Reads a text as a UUID, like `Core\Uuid::parse`, and returns `null` when the text is not a UUID.
It never throws an error, so it is the way to check whether a text is a UUID, such as an ID that
arrives in a URL or a form.

The text must be 32 hexadecimal digits in groups of 8, 4, 4, 4 and 12, joined by hyphens, such as
`f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`. Upper case and lower case letters are both allowed. The 32
digits without hyphens, a UUID in `{}` braces and a UUID after `urn:uuid:` all give `null`. The nil
UUID (all zeros) and the max UUID (all `f`) are valid.

**The examples below** read a valid UUID, show which forms give `null`, and check the ID in a
request path before the program looks anything up.
