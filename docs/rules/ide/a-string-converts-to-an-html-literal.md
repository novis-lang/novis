A cursor on a string, or a selection covering one exactly, is offered **Convert to html literal**, a
code action of kind `refactor.rewrite.htmlLiteral` that replaces the string with the ``html`…` ``
literal printing the same text (`rule:core-classes/html-literal`). On an operand of a `.` chain the
whole chain converts, since half a chain converted is a `Core\Html\Markup` concatenated with a string.

```nvs
echo "<b>" . $name . "</b> is " . Core\Str::upper("here") . '!';
echo html`<b>{$name}</b> is <?= Core\Str::upper("here") ?>!`;
```

| Written | In the literal |
|---|---|
| a double-quoted string | its text and its `$name` and `{$…}` holes as written, `\"` as `"`, every escape both grammars share kept |
| a single-quoted string | its text, with each `$` that would open a hole written `\$` |
| a backtick, a `{` that would open a hole or draw `W1012`, a `<` that would open `<?=`, `<?nvs` or `<?php` | `` \` ``, `\{`, `\x3C` |
| a variable, or a property or offset read on one, as a chain operand | a `{$…}` hole |
| any other chain operand | a `<?= … ?>` hole |

It is not offered on a heredoc or nowdoc, whose body is dedented by its closing label; on an html
literal, a template region or an attribute's argument; at an array key, a subscript, a `case` label, a
`match` condition, a constant's value or the path of a `require`, `use` or `autoload`; or on a chain
with a comment between its operands, which the rewrite would drop. Whether the result type-checks is
not asked: the preview is the review.

Before it is offered, the literal written is parsed back, and its segments must cook to the original
text and its holes must be the original expressions, in order — so a string the conversion cannot
write exactly is offered nothing rather than something else. The server computes it from the
expression alone, so every client gets the same edit, and the VS Code command
`nvs.convertToHtmlLiteral` only asks the editor to apply the action by its kind. It is never under
`source.fixAll.nvs`: a plain string echoed is escaped as text and a literal's segments are markup, so
applying it changes what the line prints, which is why it is a refactor and not a fix
(`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`).
