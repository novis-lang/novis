Work this deployment runs on a clock rather than in answer to a request.

A schedule entry names a time expression and a script file, and the server runs that file at those
times as an ordinary isolate — the same isolation a request gets, sharing nothing with anything
else. Each entry also says whether it fires once across the fleet or once on every host, which
timezone its clock is in, and what to do when a firing is still running as the next one comes round.

**In plain words:** this is the deployment's cron, written where the rest of the deployment is
written.

There is no API for it on purpose: no class to call, no registration, no attribute. A schedule is
something the deployment decided, and a registration made at runtime would be state established by
whichever request happened to run first. Everything an entry must answer is asked when the process
starts, so an entry that could never fire refuses the boot instead of being discovered months later
from the work that did not happen.

The example shows an entry, and shows that a program can neither read the schedule nor add itself to
it. The same file runs by hand with `nvs run`, which is the whole backfill and debugging story.
