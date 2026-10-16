A program ends on its own once its last line has run, and whatever started it is told all was well.

`exit` ends it sooner. Written on its own it stops the program right where it stands and still
reports success, which is what you want when there is simply nothing left to do. Give it a number
and that number becomes the status the shell reads afterwards — zero for fine, anything else for a
problem — so a program that checks or imports something can answer with it. Statuses run from 0 to
255, and a larger number wraps into that range. Give it a line of text instead and the text is
written out first, and the program still reports success.

**In plain words:** `exit` is a full stop, not an error. Nothing catches it, and no clean-up block
further up gets its turn. `die` is not a second word for it. A program that ends because an error
got away from it reports a failure instead, and says where the error came from.
