The error for something outside the program that did not happen: a file that is not there, a
directory that cannot be written to, a connection that closed halfway through a reply, a disk with
nothing left on it. It sits under `RuntimeError`, on the side of the tree that holds the world's
refusals rather than the program's own mistakes.

That is the whole difference it makes to code that catches it. A mistake of ours is fixed where it
was written; an `IOError` is answered at the call site, because the same code will work later or
against another machine. So the useful answers are the three below: fall back to something else,
report what could not be done and carry on, or try again.

Like every error it carries the message and the place it was thrown from, and it can carry the
failure it was raised in answer to, so a report can name both the summary and the cause underneath it.

**The examples below** show a helper raising one when an operation could not be done, a wrapper
keeping the original failure as the cause, and a page that stays up by falling back through the
places a value can come from.
