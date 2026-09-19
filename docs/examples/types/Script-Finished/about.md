Every script that ends early ends through this class. `Core\Script::finish()` raises it, and what
travels out is an ordinary raised value, so leaving takes the same path a thrown error takes: every
`finally` between the call and the top of the program runs, and what was opened is closed on the way
out. Nothing after the call runs.

What it is not is a failure. No `catch` takes it — not even one written for `Throwable` — because it
sits outside the error tree altogether rather than being excluded case by case. So the wrapper that
reports errors never sees a finished script as one, and nothing has to be written there to keep it
from doing so. There is nothing to read on one and no way to build one: the call is what raises it.

**The examples below** show a script that stops once its answer is written, an error report that
stays silent when a script merely finished, and a request answered from deep inside a helper with
the access log still written.
