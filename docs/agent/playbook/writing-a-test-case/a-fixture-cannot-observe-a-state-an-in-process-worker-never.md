- **A fixture cannot observe a state an in-process worker never yields inside; the tell is a counter
  polling out at `0`.** `[queue] workers` runs the job's isolate on the same core as the program
  polling `Core\Queue::stats`, so a job that returns straight away is `Claimed` only across a window
  in which the polling task is never scheduled, and polling faster cannot reach it. Make the job
  park — `examples/queue/receipt.nvs` sleeps for a beat and says why — and expect the same of any
  state a worker passes through between two of its own statements. [until: gone examples/queue/receipt.nvs:Core\Queue::stats]
