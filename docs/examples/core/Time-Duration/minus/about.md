Returns a new duration that is the duration you call it on, with another duration taken away. The
duration you call it on does not change.

Use it to find what is left of a length, for example the working time of a shift without its break,
or the time a request still has before its limit. When the duration you take away is longer, the
result is negative. Taking away a negative duration makes the result longer.

The result must fit in a duration, which is about 292 years in each direction. A difference that is
longer throws a `RuntimeError`.
