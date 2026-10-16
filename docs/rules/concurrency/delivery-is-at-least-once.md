A job may run twice, and that is stated rather than implied. The worker can die after doing the work
and before acknowledging it, or the visibility timeout can expire under load while the attempt is
still in flight; in both cases a second worker will claim the job and run it again.

Idempotency is therefore the application's obligation. `key` gives deduplication at *enqueue* time —
at most one pending job per key — and nothing else in the design pretends to give it at execution
time.

Exactly-once is not on offer, from this queue or from any honest one: systems that claim it are
describing at-least-once plus deduplication, which is what `key` and an idempotent job already are.
