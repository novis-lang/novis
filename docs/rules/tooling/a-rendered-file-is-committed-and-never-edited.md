A file a tool writes from the records is committed, so GitHub and a reader without Bun see it. It
opens with a marker naming the command that writes it, and `.gitattributes` gives it
`linguist-generated`, so a review folds it away. It is never edited by hand: the change goes into the
record or the prose fragment it is rendered from, and the file is rendered again.

The command that writes it has a check form that writes nothing and fails when the file no longer
matches its sources — `bun nv render --check` for the website's pages and data, `bun nv rules --render
--check` for the rulebook's chapters and `docs/ground-rules.md`. CI's `docs` job runs both. After a
merge conflict in a rendered file, it is rendered again from the merged
records, never merged by hand.
