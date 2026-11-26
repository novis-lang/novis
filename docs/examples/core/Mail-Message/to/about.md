Returns the recipients in the `To` header of a received message, as an array of `Core\Mail\Address`.

The addresses are in the order of the header. A message without a `To` header returns an empty
array. A group, such as `Team: a@example.com, b@example.com;`, gives one address for each member of
the group.

**Good to know:** the sender writes the `To` header. It shows who the sender named, and the mail
may have reached other people too. Each address and name is tainted.

**The examples below** list every recipient, read the members of a group, and choose a team by the
address that the message was sent to.
