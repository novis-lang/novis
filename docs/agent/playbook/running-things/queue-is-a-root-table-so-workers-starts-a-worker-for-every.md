- **`[queue]` is a *root* table, so `workers` starts a worker for every `nvs run` over that tree,
  not just the queue fixture.** There is no per-`[[app]]` spelling:
  `rule:core-classes/queue-storage-is-a-table` makes `workers` a property of the instance, so a
  worker waiting on an unreachable server charges every unrelated fixture. Grep for a root table's
  `[[app]]` twin before assuming a block only reaches the program it was written for.
  [until: reviewed 2026-09-06]
