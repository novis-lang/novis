The script a deployment runs when a request fails.

Novis reports a failure in tiers. A program may arrange its own last words, and where they worked
that is the end of it. This key names what happens when they did not: an ordinary Novis script, run
as its own isolate with the failure's report handed to it, for any failure at all — a program that
registered no handler, one whose handler failed in turn, an internal error, even a script that never
compiled.

Because it runs when the request is already out of room, it is not funded by that request. It gets a
small allotment of its own, sized once per worker, and the two keys beside it say how large.

Naming it is the operator's alone: a program able to choose who reports its failure could choose
somebody who reports nothing.

The example prints what is on duty in this checkout and shows both halves of that split.
