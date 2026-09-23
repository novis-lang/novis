- **A refusal's wording can be pinned as a substring by a test far below it in the same file.**
  `no_dialect`'s SQLite arm in `crates/nvs-stdlib/src/queue.rs` was held to the phrase "the queue has
  no statements for it yet" by `the_queues_refusal_is_only_ever_about_a_driver_that_cannot_send`, so
  an edit making the sentence *more* true failed the build. Grep a refusal's distinctive phrase
  before editing it, and when the assertion pins wording rather than the fact under it, move it to
  the fact. [until: reviewed 2026-09-10]
