Reads a UUID from its text, such as `f9168c5e-ceb2-4faa-b6bf-329bf39fa1e4`, and returns a `Uuid`.
The text must be 32 hexadecimal digits in groups of 8, 4, 4, 4 and 12, joined by hyphens. Upper case
and lower case letters are both allowed. `toString()` always gives the text back in lower case.

Other ways to write a UUID are not allowed: the 32 digits without hyphens, a UUID in `{}` braces, and
a UUID after `urn:uuid:`. For those, and for any other text, `parse` throws a `RuntimeError`. The
message quotes the start of the text.

Use `parse` when the text must be a UUID. When the text may not be a UUID, `Core\Uuid::tryParse`
returns `null` and does not throw.

**The examples below** read a UUID in upper case, show the error for a UUID in braces, and check the
ids in the rows of an import.
