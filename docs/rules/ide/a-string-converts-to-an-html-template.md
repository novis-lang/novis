A cursor on a string, or a selection covering one exactly, is offered **Convert to html template**, a
code action of kind `refactor.rewrite.htmlTemplate` that replaces the string with the ``html`…` ``
template printing the same text (`rule:core-classes/html-template`). On an operand of a `.` chain the
whole chain converts, since half a chain converted is a `Core\Html\Markup` concatenated with a string.

```nvs
echo "<b>" . $name . "</b> is " . Core\Str::upper("here") . '!';
echo html`<b>{$name}</b> is <?= Core\Str::upper("here") ?>!`;
```

| Written | In the template |
|---|---|
| a double-quoted string | its text and its `$name` and `{$…}` holes as written, `\"` as `"`, every escape both grammars share kept |
| a single-quoted string | its text, with each `$` that would open a hole written `\$` |
| a backtick, a `{` that would open a hole or draw `W1012`, a `<` that would open `<?=`, `<?nvs` or `<?php` | `` \` ``, `\{`, `\x3C` |
| a bare `$name` or `$name->prop` hole that ends its string, with another operand after it | a `{$…}` hole, so it does not run on into that operand's text |
| a variable, or a property or offset read on one, as a chain operand | a `{$…}` hole |
| any other chain operand | a `<?= … ?>` hole |

It is not offered on a heredoc or nowdoc, whose body is dedented by its closing label; on an html
template, a template region or an attribute's argument; at an array key, a subscript, a `case` label, a
`match` condition, a constant's value or the path of a `require`, `use` or `autoload`; or on a chain
with a comment between its operands, which the rewrite would drop. Whether the result type-checks is
not asked: the preview is the review.

Before it is offered, the template written is parsed back, and its segments must cook to the original
text and its holes must be the original expressions, in order — so a string the conversion cannot
write exactly is offered nothing rather than something else. The comparison uses the lexer's and the
cooker's own code, so a later change to either grammar is checked here without the conversion learning
it. The server computes it from the
expression alone, so every client gets the same edit, and the VS Code command
`nvs.convertToHtmlTemplate` only asks the editor to apply the action by its kind. It is never under
`source.fixAll.nvs`: a plain string echoed is escaped as text and a template's segments are markup, so
applying it changes what the line prints, which is why it is a refactor and not a fix
(`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`). The kind is a sub-kind narrow
enough that a client asking for it gets this action alone, and the server advertises it in
`codeActionKinds`.

- **Memory:** nothing at run time, since the action edits source and writes nothing new to the
  compiler. Per code-action request on a string: one parse of the expression under the cursor for each
  `.` above it, one of the template written, and one per enclosing array or `match` checked for a key
  position, each the size of that expression and dropped with the answer.

**Declined:**

- **A command in the VS Code extension that rewrites the text itself.** It would need its own copy of
  the lexer's escape and hole rules.
- **Converting only the string under the cursor inside a chain.** The result is a `Core\Html\Markup`
  concatenated with a string, which is not what anybody converting a chain wants.
- **Re-encoding each string from its cooked value.** It is simpler, and it turns every `\n` into a
  real line break and every `\x41` into its character, so the source changes more than the conversion
  needs.
- **Converting a heredoc by re-indenting its body.** The dedent and the escapes interact, and the case
  is rare next to strings and chains; it can be added under this rule if it is wanted.
