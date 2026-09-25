- **A goal's seed handoff opens a session with "nothing of it has landed" over work already on disk.**
  The goal switch writes the goal's seed handoff, `data/goals/<slug>.handoff.json`, over the live one,
  and that seed was written before the goal ran — goal `bigint`'s named a first item, `num-bigint` in
  `crates/nvs-stdlib/Cargo.toml`, that the previous goal's sessions had committed a dozen commits back.
  Read `git log --oneline -12` and grep the item's own anchor before taking it: a slug in a subject
  line is the one thing a seed handoff cannot know. [until: gone crates/nvs-stdlib/Cargo.toml:bigint]
