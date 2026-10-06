A run of `///` lines is one doc comment, and it attaches to the next declaration. Consecutive `///` lines
with only whitespace between them are one comment; a `///` followed by a blank line and then a declaration
is not attached to it; and a `///` attached to nothing is a diagnostic — *this doc comment is attached to
nothing; a doc comment documents the declaration it precedes.*

Attachment is what separates documentation from a note-to-self. The alternative — any comment run above a
declaration is its documentation — needs no new syntax and would start working on every file already
written; but then a note and a document are the same token, and only the author knew which was meant.
Requiring the marker
(`rule:tooling/doc-comment-is-three-slashes`) and refusing an orphan keeps the two apart.
