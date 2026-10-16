Every lane gate is a job-level `if:`, never a workflow-level path filter.

The difference matters exactly once and then permanently. A job skipped by `if:` reports success to
a required status check; a workflow filtered out by a path rule never reports at all, and a required
check on it waits forever. The second is a branch that cannot be merged, discovered by whoever turns
branch protection on months from now.
