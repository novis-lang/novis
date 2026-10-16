How long work handed off to run after the response has been sent may take, when the call names no
time of its own.

Sending an invoice, warming a cache, writing an audit row: work like that can be deferred so the
person waiting gets their answer first. The deadline is what stops a piece of it running forever and
holding on to everything its request was using. A call may name its own, and this is the default the
rest of them inherit.

A program may set it for itself, for the run it is in the middle of, which makes it one of the few
settings that is a program's business at all. The number of requests each core keeps alive for
deferred work is the other half of the same block, and that one belongs to whoever sized the
machine — so a program can shorten its own deadline but can never ask the host to hold more work.
