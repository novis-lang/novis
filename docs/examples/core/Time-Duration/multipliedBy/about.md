Returns a new duration that is the duration you call it on, multiplied by a whole number. The
duration you call it on does not change.

Use it when one length repeats, for example the length of one lesson times the number of lessons,
or a wait that doubles after each failed try. A factor of 0 gives a duration of zero. A negative
factor gives a duration that points the other way.

The result must fit in a duration, which is about 292 years in each direction. A product that is
longer throws a `RuntimeError`. The result is never shortened to make it fit.
