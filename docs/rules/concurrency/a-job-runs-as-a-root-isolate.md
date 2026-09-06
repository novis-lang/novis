A job runs as a **root isolate**: its own arena, its own budget, its own grants, sharing only compiled
code. It is the same isolate a `spawn script` builds, reached through the same door — there is no
second execution path, and no part of the enqueuing request's heap, statics or session is visible
from inside it.

A job holds the compiled unit it started with, exactly as a request and a persistent connection do,
so a redeploy mid-drain does not change what a running job is executing. A queue draining ten jobs off
one script compiles it once.

Output is captured rather than written through, because a job's `echo` landing in the middle of what
the server or the run's own script is writing is exactly the mixing capture exists to prevent.
