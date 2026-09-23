- **No `Core` member anywhere may take a `Core\Xml\Node`, so a member that needs a node's context
  cannot ask for it.** `a_parsed_tree_has_no_path_back_into_execution` in
  `crates/nvs-stdlib/src/xml.rs` sweeps every registered member's parameters for the node class and
  for `Core\Xml`, and a tree is inert data on the same terms the AST is — so the obvious shape for a
  question about an element's surroundings, handing the document back in, is refused by a test two
  thousand lines away from the row. Resolve what needs ancestors while the tree is being built,
  where the scope is still known, and carry the answer on the node.
  [until: reviewed 2026-09-17]
