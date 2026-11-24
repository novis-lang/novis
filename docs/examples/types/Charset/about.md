Names the character encoding of some text, so a conversion knows which encoding to use.

Text inside a Novis program is always UTF-8. Text from outside may use another encoding, for example
a file from an old spreadsheet program. `Core\Charset` has one case for each encoding that web
browsers support, such as `Utf8`, `Latin1`, `Windows1252`, `ShiftJis` and `Gbk`. You give the case to
`Core\Encoding::decodeText` to read such bytes as text. You give it to `Core\Encoding::encodeText` to
write your text as such bytes.

**Good to know:** a conversion is exact, or it throws an error. No `Core\Charset` value replaces the
characters that do not fit. When the encoding has no way to write a character, the conversion throws an error. The
message names that character.
