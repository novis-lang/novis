Reads a clock that only moves forward and returns a `Duration`. Use it to measure how long
something takes: read it before and after, and subtract the first reading from the second. It
replaces PHP's `hrtime`.

The value alone has no meaning. It counts from a fixed starting point in the running program, not
from a date. A later reading is never smaller than an earlier one, even when somebody changes the
system clock. For the date and time of day, use `Core\Time::now` instead. A fixed time set by a
test changes `now`, and it does not change `monotonic`.

**The examples below** measure a loop, check that readings never go back, and report every step of
a job that took longer than a limit.
