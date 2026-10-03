Returns a part of a string, chosen by its position.

`Core\Str::slice` takes a string, an offset and a length. The offset is where the part starts, and
the first character is at offset `0`. The length is how many characters the part has. Leave the
length out, or give `null`, to take everything to the end.

A negative offset counts from the end, so `-3` starts three characters before the end. A negative
length stops that many characters before the end. An offset past the end gives an empty string, and
no error is thrown.

Positions count characters as a person sees them, not bytes. "é" and a flag like 🇩🇪 are one
character each, so a slice never cuts one in half.

**Good to know:** this replaces PHP's `substr` and `mb_substr`.

related: Core\Str::at, Core\Str::before, Core\Str::after, Core\Bytes::slice
