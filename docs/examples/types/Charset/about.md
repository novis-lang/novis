Names the character encoding a piece of text is stored in, so a conversion can say which one it means.

Text inside a Novis program is always UTF-8. The world outside is not: a file exported by an old
spreadsheet, a message from a system nobody has touched in twenty years, a page from a site that
never moved on. `Core\Charset` has one case per encoding in the index that web browsers follow —
`Utf8`, `Latin1`, `Windows1252`, `ShiftJis`, `Gbk` and the rest — and you give the case to
`Core\Encoding` to read such bytes as text, or to write your text back out as those bytes.

**Good to know:** a conversion is exact or it fails. There is no case that stands for "replace what
does not fit": when a character has no spelling in the encoding you asked for, the conversion throws
and the message names the character that stopped it.
