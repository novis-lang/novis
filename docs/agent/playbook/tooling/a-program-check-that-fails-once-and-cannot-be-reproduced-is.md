- **A program check that fails once and cannot be reproduced is a race inside the fixture, and the
  error names the member that tripped over the damage rather than the one that caused it.**
  `examples/queue-purge.nvs` missed its claim window, so its own `Core\Queue::delete` removed the row
  and the next line reported `Core\Queue::status: no job 29`, which reads as a queue bug. Check
  `.loop/log.md` for whether the line repeats, then re-run the fixture a dozen times serially and
  concurrently — a bounded wait whose answer is discarded is what turns a timing miss into a false
  accusation further down. [until: reviewed 2026-10-14]
