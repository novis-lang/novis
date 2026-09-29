Returns a new duration that is the duration you call it on, with another duration added. The duration
you call it on does not change.

Use it to add lengths of time together, for example the steps of a job, or a delay that grows each time
a request is tried again. Adding a negative duration makes the result shorter, and the result can be
negative.

The result must fit in a duration, which is about 292 years in each direction. A sum that is longer
throws a `RuntimeError`.
