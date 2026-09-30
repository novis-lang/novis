Returns a new date and time with some of its parts replaced.

You name only the parts you want to change: `year`, `month`, `day`, `hour`, `minute`, `second` or
`nanos`. The other parts and the time zone stay as they are. The value you call `with` on does not
change.

The new parts must make a date that exists. 14 June with the day changed to 31 is not in the
calendar, so `with` throws a `RuntimeError`. `plus` is different: it gives the last day of the
month in that case.

**Good to know:** when the clocks move forward, one hour of that day does not exist. If the new
time is in that hour, the result is one hour later. In Berlin on 31 March 2024, a change to 02:30
gives 03:30.
