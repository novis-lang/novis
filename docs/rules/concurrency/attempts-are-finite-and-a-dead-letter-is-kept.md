A job's attempts are finite with nothing configured, because an unbounded retry is an unbounded wait
wearing a different name. Between attempts the delay grows exponentially, is jittered so a fleet does
not retry in lockstep, and is capped.

A job that exhausts its attempts **moves** to the dead-letter table, carrying its payload, every
attempt's error and its timing. The runtime never deletes it. `stats` reports the dead-letter depth
beside the pending and claimed counts, because an unwatched dead-letter table is the classic way a
queue silently loses work and a depth nobody reads is the same as no record at all.

An attempt ceiling of zero is refused at the call rather than accepted: it asks for a job dead-lettered
by the enqueue that created it, and attempts are finite, not optional.
