`Cli::displayWidth(string): uint` answers UAX #11 terminal columns for **the string the sink will
write, not the one it was handed**. The substitution of `rule:tooling/terminal-output-is-a-sink` is
applied first, so a control byte costs what its Control Picture costs — one column, never zero — and a
value cannot shrink its own measured width by carrying an escape sequence, which is what makes the
answer safe to lay a box out against.

The count is over grapheme clusters: a combining mark is no column of its own and an emoji ZWJ
sequence is one pair however many code points built it, where summing code points misaligns exactly
the table this member is asked for. The two code points the sink passes through are the two that are
not columns: `TAB` advances to the next multiple of eight, so its cost depends on where it stands, and
`LF` ends the row, so a value spanning several rows answers the width of its *widest* one — the
number a box is padded to.

It lives on `Core\Cli` and not on `Core\Str` deliberately. A column count is a third measure beside
the two `rule:types/bytes` fixed — bytes for `bytes`, grapheme clusters for `string` — and it is a
property of the renderer, not of the string; putting it on `Core\Str` would imply a string has an
intrinsic width, the confusion `rule:types/string-is-utf8` exists to remove.
