`docs/examples/` is the only copy of every example, and the website build reads it in place. Nothing
copies it into `website/`, so there is no mirror to rebuild and no second copy to fall out of date.

They live in the repository because the same sweep that tests a feature writes its examples, and a
sweep cannot write into a tree it is not allowed to touch.
