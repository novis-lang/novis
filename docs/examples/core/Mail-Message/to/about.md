Returns the recipients in the `To` header of a received message, as an array of `Core\Mail\Address`.

The addresses are in the order of the header. A message without a `To` header returns an empty
array. A group is a name, a colon, a list of addresses and a `;`. A group, such as
`Team: a@example.com, b@example.com;`, gives one address for each member of the group. An entry that is not an address, such as `<ada>`,
is not in the array.

**Good to know:** the sender writes the `To` header, and it lists the recipients the sender chose
to show. Hidden copies (`Bcc`) are not in `To`, so the mail may have reached other people too. Each
address and name is tainted. Tainted means that the value comes from outside your program, so you
must check or escape it before you use it.

**The examples below** list every recipient, read the members of a group, and choose a team by the
address that the message was sent to.
