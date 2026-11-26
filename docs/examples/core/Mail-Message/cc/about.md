Returns the recipients in the `Cc` header of a received message, as an array of `Core\Mail\Address`.

`Cc` means "carbon copy". These are the people who got a copy of the message. The addresses are in
the order of the header. A message without a `Cc` header returns an empty array.

**Good to know:** the sender writes the `Cc` header, so each address and name is tainted. Tainted
means that the value comes from outside your program, so you must check or escape it before you
use it. Escape it with `Core\Html::escape` before you put it on a page.

`Bcc` is a hidden copy. The people in `Bcc` do not appear in any header that the recipients see.

**The examples below** list the copied recipients, show a message without `Cc`, and collect the
addresses for a reply to everyone.
