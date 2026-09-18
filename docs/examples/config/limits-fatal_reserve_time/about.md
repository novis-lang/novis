The CPU time a request keeps back for its own last words.

The other half of the same safety net. A request stopped for spending too much memory still has time
to report it; a request stopped for spending too much time does not, so the handler that reports a
limit needs a slice of the clock held back for it as well as a slice of the heap. There are two
reserves and not one per limit, because those are the only two resources a handler cannot run without
spending.

Like its memory half it comes out of the budget rather than on top of it, so what ordinary work may
spend is the configured CPU time less this. Sizing it is the operator's for the same reason: a
program choosing how long its own crash report may take is choosing it at the worst possible moment.
Written nowhere, it is 50 milliseconds — enough to format a message and write it to a log target that
is being slow — and never more than a quarter of the ceiling it is carved out of.

The example prints both halves of the net together and shows which of the three numbers a program may
move.
