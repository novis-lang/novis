- **A seed corpus cannot live under `fuzz/corpus/`: `.gitignore` ignores that whole directory.** It is
  where the nightly `fuzz-smoke` cache accumulates the corpus between runs, and a file inside an ignored
  *directory* cannot be brought back by a negation pattern. Put seeds in `fuzz/seeds/<target>/` and let the
  CI step before the run copy them in — it does that for any target that has one.
  [until: gone .gitignore:/fuzz/corpus]
