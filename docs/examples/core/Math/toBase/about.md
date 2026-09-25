`Core\Math::toBase` writes an `int` in another base and returns the digits as a `string`. The base
is from `2` to `36`. Base 2 is binary, base 16 is hexadecimal, and digits above `9` are the lower
case letters `a` to `z`. This replaces PHP's `decbin`, `dechex`, `decoct` and `base_convert`.

The result has no prefix such as `0x` and no leading zeros. Zero is `"0"`. A negative number
starts with `-`. A base outside `2` to `36` throws a `RuntimeError`. PHP's `base_convert` returns
`0` for such a base.

`Core\Math::fromBase` does the opposite. It reads the digits and returns the `int`.

**The examples below** show binary and hexadecimal numbers, then which bases are allowed, then how
to turn a numeric id into a short code for a link.
