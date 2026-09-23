- **A member the dossier says owes a Rust test can already have one, along with the reader helper you
  are about to add.** `arr.rs`'s test module runs past two thousand lines, so
  `count_by_counts_each_bucket_in_first_occurrence_order` and its `counts_of` sat far below where a
  new `countBy` test goes, and writing both cost a compile cycle on `E0428`. `grep -n
  nvs_core_<class>_<member> <file>` first — a hit inside the test module is a `covers:` marker to add
  above an existing `#[test]`, never a second test. [until: reviewed 2026-09-20]
