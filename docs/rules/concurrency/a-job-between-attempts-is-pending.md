`Queue\State` is `Pending`, `Claimed`, `Succeeded`, `Dead` and `Cancelled`. There is no `Failed`,
because a failed attempt is retried: a job between attempts is `Pending` with its backoff still to
elapse, and it is indistinguishable from one that has never run — which is correct, since both are
waiting to be claimed.

So "did this job fail" is a question about `Dead`, and the answer is in the dead-letter table
(`rule:concurrency/attempts-are-finite-and-a-dead-letter-is-kept`) rather than in a state the job
passes through.

The state is a closed integer type (`rule:enums/closed-integer-type`), not the ordinal the row stores
it as: a program compares against a case rather than a magic number, and a `Queue\State` is not
interchangeable with another `Core` enum that happens to share its ordinals.
