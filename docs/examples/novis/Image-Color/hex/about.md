Makes a colour from text written in hex, such as `"#ff8800"`.

`Color::hex($text)` returns a `Color`. The text has two hex digits for each channel: red, green,
blue and, if you want, alpha. So `"#ff8800"` is red 255, green 136 and blue 0. With a fourth pair,
`"#ff880080"`, the alpha is 128 out of 255, which is about `0.5`. Without it, the alpha is `1.0`.

The short forms `"#f80"` and `"#f808"` have one digit per channel, and each digit is written twice:
`"#f80"` is the same as `"#ff8800"`. The `#` is optional, and upper and lower case letters both
work.

Any other text throws a `LogicError`. This includes the wrong number of digits, such as `"#12345"`,
and characters that are not hex digits, such as `"#gg0000"`.

**Good to know:** use `Color::rgba` when you already have the channels as numbers.
