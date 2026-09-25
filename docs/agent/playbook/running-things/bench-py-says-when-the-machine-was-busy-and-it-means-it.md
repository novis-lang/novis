- **`bun nv bench` says when the machine was busy, and it means it.** A sweep whose min and median
  differ by more than a quarter on a case prints a note naming that case, and on `00-baseline`, which
  is subtracted from every row, it means that run's ratios are all shifted. Re-run before quoting,
  and do not reason about a small row move from a sweep carrying that note.
  [until: gone tools/nv/cmd/bench.ts:00-baseline]
