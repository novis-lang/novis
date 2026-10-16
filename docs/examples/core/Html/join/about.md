Puts a list of HTML pieces together into one piece, with a separator between each two.

Each piece in the list is a `Core\Html\Markup` value, and so is the separator. `Core\Html::join`
writes the pieces in the order of the list. The separator goes between two pieces, and never before
the first piece or after the last one. An empty list gives an empty `Markup`.

`join` does not escape anything. Every piece is already `Markup`, so its text was escaped or written
by you before. This is the same as adding the pieces with `+`, but a long list needs only one call.

**Good to know:** the list must contain `Markup` values. A list of plain strings does not compile,
because those strings are not escaped yet. Use `Core\Html::escape` on each string first.
