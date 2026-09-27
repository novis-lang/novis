Returns the position in the text where a match starts.

The first character of the text is at position `0`. The position counts characters, the same way a
person counts them, and not bytes. So an accented letter or an emoji before the match counts as
one. Every `Core\Str` method counts positions the same way, so you can give this position to
`Core\Str::slice` or to the `from` option of `Core\Regex::match`. PHP's `PREG_OFFSET_CAPTURE` flag
gives a position in bytes instead.

The position is where the whole match starts, which is also where group `0` starts.

The examples show where a word starts, a position after accented letters, and marking the place of
an error in a line of input.
