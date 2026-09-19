The error for a call that never comes back. A function that calls itself — a walk over a tree, a
parser over nested text — is stopped once the calls are stacked deeper than any working program
needs, rather than being left to exhaust the machine and take the whole process with it.

It sits under `RuntimeError`, and it is an ordinary one: a `catch` takes it, and by the time the
handler runs the deep calls are gone, so the program carries on with the room it started with. That
is what makes a recursive walk over data somebody else supplied safe to attempt — the bad case costs
one request, not the process.

Raising one yourself is often better still. Your own limit — three levels of a menu, thirty of a
reply thread — is reached long before the language's, and the message can name what was too deep.

**The examples below** show a thread whose replies point back at themselves, a menu with a depth
budget of its own, and an import that drops the one feed it cannot finish and keeps going.
