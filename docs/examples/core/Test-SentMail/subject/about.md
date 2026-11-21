`subject()` returns the subject line of a message. In a test, `Core\Test::sentMail()` returns one
`Core\Test\SentMail` for each message, and `subject()` is one of its methods.

The subject is the `$subject` string that your program gave to `Core\Mail::send`. It is exactly
that string. No text is added to it, and no text is removed from it.

Use `subject()` to check the line a person sees first in their inbox. A good subject has the
information that the person looks for, such as an order number or the name of a ticket.

**The examples below** show a subject with an order number in it, a long product name that is
shortened to fit the subject, and a support ticket form whose title goes into the subject.
