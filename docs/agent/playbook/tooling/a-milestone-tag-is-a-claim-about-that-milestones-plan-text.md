- **A milestone tag is a claim about that milestone's plan text, and the plan can refuse the gap
  rather than carry it.** `crates/nvs-stdlib/src/random.rs`'s "not reseeded on `fork`" pointed at
  M7's unbuilt service registration, but `docs/plan/m7.md:62` writes a `Type=notify` unit — the
  systemd type that does not daemonize — and `rule:core-classes/process-is-argv-only` makes every
  child an exec, so M7 is where that hazard is ruled out rather than scheduled. Read the plan
  paragraph a gap points at before tagging it with that milestone, and when the paragraph refuses
  the item's premise the item is a decision that moves above `# Known gaps`.
  [until: reviewed 2026-09-10]
