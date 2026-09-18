The memory a request keeps back for its own last words.

Every request runs under a heap ceiling, and reaching it ends the request. The one piece of code that
should still run at that moment is the handler that says what happened — and by then there is nothing
left to run it in. This directive is the slice held back from the ceiling for exactly that: ordinary
work never sees those bytes, and the handler gets them when a limit is what ended the request.

The slice is carved out of the request's own budget rather than added to it, so reserving more leaves
programs a little less to work with. How much to keep back is the operator's decision and never the
program's: a script sizing its own safety net, moments before it needs one, is the case where the
choice most clearly belongs to somebody else. Written nowhere, it is a megabyte, and it is never more
than a quarter of the ceiling it comes out of — a short budget stays mostly program.

The example shows what a request starts with, what is held back from it, and what happens when the
program asks to change either one.
