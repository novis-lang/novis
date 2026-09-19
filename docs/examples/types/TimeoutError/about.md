The error for something that did not answer in time: a connection that never opened, a query still
running when its limit was reached, a cache that went quiet. It sits under `RuntimeError`, beside
the other refusals that come from outside the program.

It is the one failure that tells you nothing about what happened. A refused connection did not
happen; a timed-out one may have arrived, been carried out, and only lost its answer on the way
back. So the safe answer to a timeout is rarely to do the thing again — it is to ask whether it was
already done, unless doing it twice is harmless.

A deadline is also worth setting in your own code, so that work nobody is waiting for any more stops
instead of holding a request open. An error raised that way carries what it was waiting for in its
message, like any other.

**The examples below** show a helper that gives an operation a budget and gives up when it is spent,
a payment whose timeout is answered by asking rather than by paying again, and one deadline shared
across the several steps of a single request.
