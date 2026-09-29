One workflow file holds every CI job, and a schedule trigger plus a `full` flag decide which of them
a given run needs.

The **push lane** covers a push to `main` and every pull request; each job runs under
its own `if:`. The **deep lane** — the nightly schedule, a manual dispatch, and the release gate
calling this same workflow — forces every gate true, so every job runs. Always-on are the document
checks, which cost seconds.

A job is never defined twice. A second copy of a suite drifts, and it drifts silently because
nothing compares the copies, which is why the release gate calls this workflow rather than restating
it — and why a fast lane can never be bought by deleting a job, since that deletes it from the
release gate in the same edit.

Nothing has left the suite. What a push skips is work whose answer its own diff cannot change.
