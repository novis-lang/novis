`docs/examples/` is authoritative, and the website's example tree is a mirror rebuilt from it. The
site's own rule is unchanged — tool-owned files are regenerated, human-owned files are never
overwritten — and this simply makes the example tree one of the tool-owned ones.

They live in the repository because the same sweep that tests a feature writes its examples, and a
sweep cannot write into a tree it is not allowed to touch.
