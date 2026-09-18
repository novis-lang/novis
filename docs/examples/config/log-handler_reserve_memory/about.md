The memory the script that reports a failure is allowed to use.

A failing request has nothing to lend — being out of room is often what went wrong — so the script
named beside this key does not spend its budget. It gets an allotment of its own, sized once per
worker at startup, and this is how large.

Sixteen megabytes where nobody writes otherwise, far more than the slice `[limits]` keeps for a
handler the program already had loaded: this one compiles and runs a whole script before it can say
anything.

Sizing it belongs to whoever runs the deployment, for the reason naming the script does: a program
able to shrink the room its own report is written in could make that report fail too, and a failure
nobody hears about is what the whole ladder exists to prevent.

The example prints both halves of the allotment beside the request-side slice they are confused
with.
