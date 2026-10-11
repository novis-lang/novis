`decimal` is a scalar: a sign, a 96-bit unsigned mantissa, and a scale of 0 to 28 giving the digits
after the point. Its value is `(-1)^sign × mantissa × 10^-scale` — roughly 29 significant digits. It
is register-pair sized, allocation-free and refcount-free.

It is a **scalar rather than a class** because Novis has no operator overloading
(`rule:classes/comparable` refuses a cross-class one), so a `Core\Decimal` class could never take part
in `+` and money arithmetic would be `$price->mul($qty)->add($shipping)` forever. Unreadable money
arithmetic is what drives a developer to `float` and its rounding, so the ergonomics are the
correctness question. A class also could not be a compile-time constant (`const decimal VAT = 0.19;`),
an attribute payload or a literal type; its only precision-safe constructor would take a string,
since `Decimal::of(19.99)` has lost the value before the call; and every intermediate of an
expression would be a heap allocation with a refcount. `Core\Db` lands `DECIMAL` and `NUMERIC`
columns in it without loss (`rule:core-classes/db-column-types`).

A `decimal` compares with `int`, `uint` and `float` under `< <= > >= ==`, exactly over the full range
of both, although it does no arithmetic with a `float`: an exact comparison is always computable where
a common arithmetic type is not, the same reason `int` compares with `uint`.

It is deliberately **not** arbitrary precision. World GDP in cents is 17 digits; Bitcoin to satoshis
is 16. A whole number beyond the range is a `Core\BigInt`, and no type holds a fraction beyond it.
The boundary is stated here so that nobody has to discover it.

Scale is carried for rendering and does not affect equality or hashing: `1.10 == 1.1000` is true, and
`19.90` renders `"19.90"`. **Division is the one operation that may be inexact**, and its policy is
fixed in the language and not configurable: round half to even, at the maximum scale the result
admits, and `decimal / decimal` is always a `decimal`, never the union `int / int` gives. There is no
`bcscale()` equivalent and never will be: ambient precision read by unrelated later code is the
shape `rule:statements/static-is-a-member-modifier` and `rule:security/no-cross-request-state`
already close. Where rounding is business logic it is
said out loud — `Core\Decimal::divExact()` throws unless the quotient is exact,
`::divRound($scale, $mode)` names both, and `::allocate($amount, $ratios)` splits a sum into parts
that add back to it exactly.

`bcmath` and `gmp` are retired rather than ported, because they conflated two unrelated capabilities:
exact fractional arithmetic at human magnitudes, which is `decimal`, and arbitrary-magnitude integers
wearing a decimal API, which is `Core\BigInt`. `decimal` arithmetic runs in registers instead of
digit by digit over strings. `gmp` is not replaced by binding GMP itself, which is C and LGPL.

`decimal` is not an enum backing type (`rule:enums/one-backing-type`), and array keys are unaffected —
every key is a `string` already (`rule:types/arrays`).

- **Memory:** 16 bytes per value against 8 for a `float`, and less than the heap object and refcount a
  boxed decimal would hold.

**Declined:**
- **`Core\Decimal` as an ordinary class.** Without operator overloading it is permanently more verbose
  than the wrong answer, and it forecloses constants, literal types and attribute payloads. Adding the
  scalar later would mean rewriting every money-handling program written against the class.
- **A `money` type carrying a currency.** Currency is application domain, and what USD ÷ USD is has no
  single right answer. A userland `Money` class layers on `decimal` correctly.
- **A scale in the type, `decimal<2>`.** It needs type-level integers, which nothing else in Novis has,
  and it makes every signature that accepts some decimal either generic or wrong.
- **Arbitrary precision as the scalar.** A heap mantissa puts an allocation on every intermediate of
  every money expression, for a range essentially no web application needs.
- **IEEE 754-2008 `decimal128`.** The mature implementations are C, failing
  `rule:packaging/a-c-dependency-answers-two-questions`, and its cohorts (several representations of one
  value) add subtlety that buys nothing monetary.
