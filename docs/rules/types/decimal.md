`decimal` is a scalar: a sign, a 96-bit unsigned mantissa, and a scale of 0 to 28 giving the digits
after the point. Its value is `(-1)^sign × mantissa × 10^-scale` — roughly 29 significant digits. It
is register-pair sized, allocation-free and refcount-free, and it costs **16 bytes per value** against
8 for a `float`.

It is deliberately **not** arbitrary precision. World GDP in cents is 17 digits; Bitcoin to satoshis
is 16. A whole number beyond the range is a `Core\BigInt`, and no type holds a fraction beyond it.
The boundary is stated here so that nobody has to discover it.

Scale is carried for rendering and does not affect equality or hashing: `1.10 == 1.1000` is true, and
`19.90` renders `"19.90"`. **Division is the one operation that may be inexact**, and its policy is
fixed in the language and not configurable: round half to even, at the maximum scale the result
admits. There is no `bcscale()` equivalent and never will be. Where rounding is business logic it is
said out loud — `Core\Decimal::divExact()` throws unless the quotient is exact,
`::divRound($scale, $mode)` names both, and `::allocate($amount, $ratios)` splits a sum into parts
that add back to it exactly.

`bcmath` and `gmp` are retired rather than ported, because they conflated two unrelated capabilities:
exact fractional arithmetic at human magnitudes, which is `decimal`, and arbitrary-magnitude integers
wearing a decimal API, which is `Core\BigInt`.

`decimal` is not an enum backing type (`rule:enums/one-backing-type`), and array keys are unaffected —
every key is a `string` already (`rule:types/arrays`).
