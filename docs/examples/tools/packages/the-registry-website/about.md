The registry website finds packages and shows what the registry knows about each one. A search
matches your words in the name of a package, in its readme and in its doc comments. You choose the
order: `default`, `date`, `activity`, `downloads` or `name`.

The `default` order uses a score from 0 to 1. Half of it comes from facts that the registry saw
itself, such as the months with a release and the share of the API with a doc comment. The other
half comes from the downloads of the last 90 days. The formula is the file
`apps/registry/src/Ranking.nvs`.

The page of a package shows each version with its state, the commit it was built from, its owners,
its retractions and every fact of its score. The page of a version also shows its capabilities, the
changes to its API and the files in its archive.

**Good to know:** the dates and commits in a git repository do not change the score, because
anybody can write them. `GET /api/search?q=<words>&sort=<order>` returns the same search as JSON.
