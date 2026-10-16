No `Core` member reads state the call site did not give it. There is no process or per-request default
timezone, no locale, no internal array pointer and no error global; a thread-per-core runtime cannot have
any of them without leaking one request's setting into the next.

So a `Zone` is an explicit argument at every instant↔calendar conversion, translation takes its locale as
an argument, and the members that would have read a global read a parameter instead. The same argument that
refuses `setlocale` — process-wide C state that silently changes what a later call answers — refuses every
smaller version of it.

The cost is stated rather than hidden: every date formatting call names a zone, which is correct and is more
typing than an ambient default zone for the common case. What it buys is that a member's answer is a function of its arguments,
which is also what makes a call reviewable and a compile-time fold possible.
