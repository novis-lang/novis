Returns the addresses in the `Reply-To` header of a received message, as an array of `Core\Mail\Address`.

A sender writes `Reply-To` when an answer should go to another address than the `From` address.
A contact form on a website is a common case: the mail comes from the website, and the reply goes
to the visitor. A message without a `Reply-To` header returns an empty array, and the reply then
goes to the `From` address.

**Good to know:** the sender writes this header, so each address and name is tainted. A reply to
a stranger's `Reply-To` goes wherever that stranger chose.

**The examples below** read where a reply goes, use the sender when there is no `Reply-To`, and
answer the visitor of a contact form.
