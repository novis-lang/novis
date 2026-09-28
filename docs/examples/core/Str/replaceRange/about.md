Replaces a part of a string, chosen by its position, with a new text.

`Core\Str::replaceRange` takes a string, an offset, a length and a replacement. The offset and the
length choose the part to replace, in the same way as `Core\Str::slice`: the offset is where the part
starts, and the length is how many characters it has. A negative offset counts from the end. A
negative length stops that many characters before the end, and `null` means "to the end".

An empty replacement deletes the part. A length of `0` deletes nothing, so the replacement is
inserted at the offset. Positions count characters as a person sees them, not bytes, so "é" is one
character.

To replace a text wherever it appears, use `Core\Str::replace`.

**Good to know:** this replaces PHP's `substr_replace`.
