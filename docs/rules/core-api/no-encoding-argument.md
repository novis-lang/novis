No member taking a `string` takes an encoding argument. UTF-8 is the type's guarantee
(`rule:types/string-is-utf8`), so there is nothing for such an argument to select.

All conversion happens at the `bytes`↔`string` boundary, in `Core\Encoding`, where it can fail honestly:
decoding arbitrary bytes into a `string` is the operation that can go wrong, and it is the one that says
so. No member therefore exists twice, once for bytes and once for characters, because the byte version
cannot be trusted with text.

The cost lands on programs that genuinely handle non-UTF-8 text: they hold `bytes`
(`rule:types/bytes`) until they have decided what the encoding is, and the decision is written where it is
made instead of defaulted per call.
