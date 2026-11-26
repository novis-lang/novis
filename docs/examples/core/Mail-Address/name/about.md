Returns the name that is shown for an email address, such as `Ada Lovelace`. An address with no
name gives an empty string.

For an address that `Core\Mail\Address::of` made, the name is its second argument. For an address
of a received message, the name is the text before the address in the header, such as
`Ada Lovelace` in `Ada Lovelace <ada@example.org>`. A name with letters that are not ASCII is often
an encoded word (the way a header writes non-ASCII text), such as `=?UTF-8?Q?...?=`. `name()`
decodes it.

`name()` returns a `tainted string`. Tainted means that the value comes from outside your program,
so you must check or escape it before you use it. Escape it with `Core\Html::escape` before you put
it on a page. It cannot be part of a file path or an SQL query without a check.

The sender of a message can write any name, so the name does not prove who sent the message.

**The examples below** show how to greet the sender by name, an address with no name, and an
encoded name.
