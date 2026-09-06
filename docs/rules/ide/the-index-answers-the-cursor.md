`SyntaxIndex` is built by one walk over the tree and answers `at(offset) -> NodePath`: the innermost node
containing the offset plus its ancestors. Hover, definition and completion each need a different depth of
that path, and `selectionRange` is the ancestor list itself, so the request is a projection of the index
rather than a feature built on top of it.

The index is rebuilt per analysis. Making it incremental belongs with the rest of incrementality, and the
ancestor paths are what would make item-level caching expressible if `rule:ide/a-full-reanalysis-stays-under-a-bound`
ever fails.
