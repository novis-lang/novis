Consecutive `use` declarations are sorted lexicographically by their full path, ascending and
case-sensitive, with no blank line between them. Exactly one blank line separates that block from the
`namespace` line above and from the first real declaration below.

A `use` declaration names exactly one imported path, so there is no grouped `use A\{B, C};`
form to order or to expand. This is the only reordering the formatter performs anywhere: class members
keep the order their author wrote (`rule:tooling/fmt-never-reorders-members`), because an import's
position is not observable and a member's is.
