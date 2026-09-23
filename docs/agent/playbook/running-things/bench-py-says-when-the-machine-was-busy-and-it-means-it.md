- **`bench.py` says when the machine was busy, and it means it.** A sweep whose min and median
  differ by more than a quarter on `00-baseline` prints a note, and since the baseline is subtracted
  from every row, that run's ratios are all shifted. Re-run before quoting, and do not reason about
  a small row move from a sweep carrying that note. [until: reviewed 2026-09-06]
