Returns the name that is shown for an email address, such as `Ada Lovelace`. An address with no
name gives an empty string.

For an address that `Core\Mail\Address::of` made, the name is its second argument. For an address
of a received message, the name is the text in front of the address in the header, such as
`Ada Lovelace` in `Ada Lovelace <ada@example.org>`. An encoded name, such as `=?UTF-8?Q?...?=`, is
decoded.

`name()` returns a `tainted string`. Tainted means that the value can come from outside your
program. The sender of a message writes any name they like, so the name does not prove who sent
it. Escape it with `Core\Html::escape` before you put it on a page.

**The examples below** show how to greet the sender by name, an address with no name, and an
encoded name.
