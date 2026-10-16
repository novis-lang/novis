`<?nvs` is the only code-mode open tag. The lexer still recognises `<?php` and switches to code mode on it,
purely so the parser can name the fix instead of misreading the rest of the file as inline HTML: the parser
reports `E0229` every time it consumes that token — at file start or at a mid-file reopen alike — and then
parses the code that follows normally, so nothing after the tag is swallowed.

```nvs
<?php echo 1; ?>          // tag rejected, `echo 1;` still parses as code
<?nvs echo 1; ?>          // unaffected
if ($x) { ?>html<?nvs }   // unaffected — reopening mid-block was always legal
```

`<?=` is untouched: it was never a spelling of `<?nvs`, it is sugar for `<?nvs echo`.

`<?php` and `<?nvs` always produced the same token past the parser, so the second tag preserved no
behaviour, only a spelling, and `rule:statements/nothing-gets-a-second-name` rules out a second spelling
of one construct. Converting a file that opens with `<?php` is a five-character substitution with
nothing to analyse.

One file shape reaches code mode without a tag, and it is not a second spelling of one. A file whose first
two bytes are `#!` continues in code mode as if `<?nvs` stood there; writing the tag in a shebang file
before any `?>` is `E0009`.

**Declined:**
- **`<?php` as a permanent alias of `<?nvs`.** It differs in nothing, so it buys only a second thing to
  teach, the trade `rule:types/no-legacy-cast` and `rule:expressions/no-keyword-logical-operators` also
  refuse.
