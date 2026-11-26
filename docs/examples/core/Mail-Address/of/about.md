`Core\Mail\Address::of` returns an email address with a name, such as
`"Ada Lovelace" <ada@example.com>`. A mail program shows the name in place of the address.

Pass the result to `Core\Mail::send` as a recipient, a `cc`, a `bcc` or the `replyTo` address.

The first argument is the address only. `of` checks it at once, with the same checks as `send`. An
empty address, or an address with a space, a line break or a second `@`, throws a `RuntimeError`.
A name inside the address string, such as `Ada <ada@example.com>`, also throws. Pass the name as
the second argument.

The name can be any text, including text a user typed. It is encoded before it is sent, so a line
break or a comma in it is part of the name. It cannot add a header or a recipient. An empty name
sends the address alone.

A domain with letters that are not ASCII, such as `bücher.example`, is sent in its `xn--` form.

**The examples below** show a recipient with a name, a name inside the address, and a test of a
name from a form.
