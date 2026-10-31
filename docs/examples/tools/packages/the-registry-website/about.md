The registry website finds packages and shows what the registry knows about each one. A search
matches your words in the name of a package, in its readme and in its doc comments. You choose the
order: `default`, `date`, `activity`, `downloads` or `name`.

The `default` order uses a score from 0 to 1. Half of it comes from facts that the registry saw
itself. The other half comes from the downloads of the last 90 days. The formula is the file
`apps/registry/src/Ranking.nvs`.

The page of a package shows each version with its state, its commit, its owners and every fact of
its score. You create an account on `/signup` with a passkey and sign in on `/signin`. An owner
also sees forms that unlist, retract, send an advisory, move the package and release a pending
version.

**Good to know:** the dates and commits in a git repository do not change the score, because
anybody can write them. `GET /api/search?q=<words>&sort=<order>` returns the same search as JSON.
